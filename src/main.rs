use std::{
    fs::File,
    io::Read,
    env,
    thread,
    time::Duration,
    ops::{Index, IndexMut}
};

use sdl2::{
    Sdl,
    EventPump,
    pixels::{PixelFormatEnum, Color},
    event::Event,
    video::{WindowContext, Window},
    render::{Canvas, TextureCreator, Texture}
};


struct WindowHolder
{
    ctx: Sdl,
    canvas: Canvas<Window>
}

impl WindowHolder
{
    pub fn new(width: u32, height: u32) -> Self
    {
        let ctx = sdl2::init().unwrap();

        let video = ctx.video().unwrap();

        let window = video.window("binary visualizer!", width, height)
            .build()
            .unwrap();

        let canvas = window.into_canvas()
            .build()
            .unwrap();

        Self{ctx, canvas}
    }

    pub fn events(&self) -> EventPump
    {
        self.ctx.event_pump().unwrap()
    }

    pub fn texture_creator(&self) -> TextureCreator<WindowContext>
    {
        self.canvas.texture_creator()
    }

    pub fn draw(&mut self, texture: &Texture)
    {
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();

        self.canvas.copy(texture, None, None).unwrap();

        self.canvas.present();
    }
}

struct DrawerWindow<'a>
{
    events: EventPump,
    window: WindowHolder,
    texture: Texture<'a>
}

impl<'a> DrawerWindow<'a>
{
    pub fn new(
        window: WindowHolder,
        texture_creator: &'a TextureCreator<WindowContext>,
        image: &Image
    ) -> Self
    {
        let texture = texture_creator
            .create_texture_static(
                PixelFormatEnum::RGBA32,
                image.width() as u32,
                image.height() as u32
            ).unwrap();

        let mut this = Self{events: window.events(), window, texture};

        this.update(image);

        this
    }

    pub fn update(&mut self, image: &Image)
    {
        let data = image.data_raw();
        self.texture.update(None, &data, image.width() * 4).unwrap();
    }

    pub fn wait_exit(mut self)
    {
        loop
        {
            for event in self.events.poll_iter()
            {
                match event
                {
                    Event::Quit{..} => return,
                    _ => ()
                }
            }

            self.window.draw(&self.texture);

            thread::sleep(Duration::from_millis(1000 / 60));
        }
    }
}

struct Image<T=Color>
{
    data: Vec<T>,
    width: usize,
    height: usize
}

impl<T> Image<T>
where
    T: Clone
{
    pub fn new(width: usize, height: usize, c: T) -> Self
    {
        Self{
            data: vec![c; width * height],
            width,
            height
        }
    }

    pub fn map<F, U>(self, f: F) -> Image<U>
    where
        F: FnMut(T) -> U
    {
        Image{
            data: self.data.into_iter().map(f).collect(),
            width: self.width,
            height: self.height
        }
    }

    #[allow(dead_code)]
    pub fn unhilbertify(&mut self)
    {
        assert_eq!(self.width, self.height);

        let size = self.width;
        let curve = HilbertCurve::new(size);

        self.remap_positions(|index|
        {
            let pos = curve.value_to_point(index);

            Self::to_index_assoc(size, pos)
        });
    }

    #[allow(dead_code)]
    pub fn hilbertify(&mut self)
    {
        assert_eq!(self.width, self.height);

        let size = self.width;
        let curve = HilbertCurve::new(size);

        self.remap_positions(|index|
        {
            let pos = Self::index_to_pos_assoc(size, index);

            curve.point_to_value(pos)
        });
    }

    fn remap_positions(&mut self, mut f: impl FnMut(usize) -> usize)
    {
        let mut output = self.data.clone();

        self.data.iter().enumerate().for_each(|(i, value)|
        {
            let new_position = f(i);

            output[new_position] = value.clone();
        });

        self.data = output;
    }

    pub fn width(&self) -> usize
    {
        self.width
    }

    pub fn height(&self) -> usize
    {
        self.height
    }

    pub fn to_index(&self, pos: Pos2<usize>) -> usize
    {
        Self::to_index_assoc(self.width, pos)
    }

    pub fn to_index_assoc(width: usize, pos: Pos2<usize>) -> usize
    {
        pos.y * width + pos.x
    }

    pub fn index_to_pos_assoc(width: usize, index: usize) -> Pos2<usize>
    {
        Pos2{
            x: index % width,
            y: index / width
        }
    }
}

impl Image<Color>
{
    pub fn data_raw(&self) -> Vec<u8>
    {
        self.data.iter().flat_map(|c|
        {
            [c.r, c.g, c.b, c.a]
        }).collect()
    }
}

impl<T> Index<Pos2<usize>> for Image<T>
where
    T: Clone
{
    type Output = T;

    fn index(&self, index: Pos2<usize>) -> &Self::Output
    {
        &self.data[self.to_index(index)]
    }
}

impl<T> IndexMut<Pos2<usize>> for Image<T>
where
    T: Clone
{
    fn index_mut(&mut self, index: Pos2<usize>) -> &mut Self::Output
    {
        let index = self.to_index(index);

        &mut self.data[index]
    }
}

#[derive(Debug, Copy, Clone)]
struct Pos2<T>
{
    x: T,
    y: T
}

struct HilbertCurve
{
    order: usize
}

impl HilbertCurve
{
    pub fn new(size: usize) -> Self
    {
        let mut order = 0;

        let mut current = size;
        while current > 0
        {
            current /= 2;

            order += 1;
        }

        order -= 1;

        if current != 0
        {
            panic!("size must be a power of 2");
        }

        Self{order}
    }

    fn rotate(&self, mut pos: Pos2<usize>, check: Pos2<usize>, value: usize) -> Pos2<usize>
    {
        if check.y != 0
        {
            return pos;
        }

        if check.x == 1
        {
            pos.x = value - 1 - pos.x;
            pos.y = value - 1 - pos.y;
        }

        Pos2{x: pos.y, y: pos.x}
    }

    #[allow(dead_code)]
    pub fn point_to_value(&self, mut pos: Pos2<usize>) -> usize
    {
        let n = 2_usize.pow(self.order as u32);

        (0..self.order).rev().map(|s|
        {
            let s = 2_usize.pow(s as u32);

            let rx = ((pos.x & s) > 0) as usize;
            let ry = ((pos.y & s) > 0) as usize;

            pos = self.rotate(pos, Pos2{x: rx, y: ry}, n);

            s * s * ((3 * rx) ^ ry)
        }).sum()
    }

    pub fn value_to_point(&self, mut value: usize) -> Pos2<usize>
    {
        let mut pos = Pos2{x: 0, y: 0};

        for s in 0..self.order
        {
            let s = 2_usize.pow(s as u32);

            let rx = (value / 2) & 1;
            let ry = (value ^ rx) & 1;

            pos = self.rotate(pos, Pos2{x: rx, y: ry}, s);

            pos.x += s * rx;
            pos.y += s * ry;

            value /= 4;
        }

        pos
    }
}

fn put_points(image: &mut Image<u32>, bytes: Vec<u8>)
{
    for (&x, &y) in bytes.iter().zip(bytes.iter().skip(1))
    {
        image[Pos2{x: x as usize, y: y as usize}] += 1;
    }
}

fn main()
{
    let input_path = env::args().nth(1).expect("provide input file plz");
    let mut input_file = File::open(&input_path).unwrap_or_else(|err|
    {
        panic!("provide a valid file, cant open: {} ({err})", input_path)
    });

    let mut input_bytes = Vec::new();
    input_file.read_to_end(&mut input_bytes).unwrap();

    let image_size = 256;

    let mut image: Image<u32> = Image::new(image_size, image_size, 0);
    let top_value = input_bytes.len() / (image_size * image_size);

    put_points(&mut image, input_bytes);

    let image = image.map(|v|
    {
        let v = v as f64 / top_value as f64;

        let c = (v * 256.0).clamp(0.0, 255.0) as u8;

        Color::RGB(c, c, c)
    });

    let scale = 2;

    let holder = WindowHolder::new(image_size as u32 * scale, image_size as u32 * scale);

    let texture_creator = holder.texture_creator();

    let window = DrawerWindow::new(holder, &texture_creator, &image);

    window.wait_exit();
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn inverse_hilbert()
    {
        let n = 512;

        let curve = HilbertCurve::new(n);

        let total = n * n;
        for i in 0..total
        {
            let point = curve.value_to_point(i);

            assert_eq!(curve.point_to_value(point), i);
        }
    }
}
