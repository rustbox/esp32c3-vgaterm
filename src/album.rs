
use alloc::vec::Vec;

use crate::video;

pub struct Image<'a> {
    data: &'a [u8],
    height: usize,
    width: usize,
}

pub struct Album<'a> {
    images: Vec<Image<'a>>,
    current: usize,
}

impl<'a> Album<'a> {
    pub fn new(images_raw: &[&'a [u8]]) -> Album<'a> {
        let mut images = Vec::new();
        for img in images_raw {
            let image = Image {
                data: img,
                height: 400,
                width: 640,
            };
            images.push(image);
        }
        Album {
            images,
            current: 0,
        }
    }

    ///
    /// Basically we want to
    /// 1) Display an image
    /// 2) Wait some amount of time
    /// 3) Begin transition to the next photo with an effect
    /// 4) Display the next image
    /// 5) Wait
    ///
    #[allow(dead_code)]
    fn update(&mut self) {}

    pub fn next(&mut self) {
        self.current = self.current.wrapping_add(1) % self.images.len();
    }

    pub fn show(&self) {
        let image = &self.images[self.current];
        for y in 0..image.height {
            for x in 0..image.width {
                let i = image.width * y + x;
                video::set_pixel(x, y, image.data[i]);
            }
        }
    }
}

