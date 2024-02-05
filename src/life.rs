use alloc::{boxed::Box, vec::Vec};
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Point, Size},
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StyledDrawable},
};

use crate::{
    color::{self, Rgb3},
    video::{self, HEIGHT, WIDTH},
};

const SIZE: usize = 1024;

#[derive(Debug)]
pub enum CellState {
    Live,
    Dead,
}

impl CellState {
    fn is_live(&self) -> bool {
        match self {
            CellState::Live => true,
            CellState::Dead => false,
        }
    }

    fn is_dead(&self) -> bool {
        match self {
            CellState::Live => false,
            CellState::Dead => true,
        }
    }
    //
    // Any live cell with fewer than two live neighbours dies, as if by underpopulation.
    // Any live cell with two or three live neighbours lives on to the next generation.
    // Any live cell with more than three live neighbours dies, as if by overpopulation.
    // Any dead cell with exactly three live neighbours becomes a live cell, as if by reproduction.
    //
    fn next(&self, neighbors: &[GridLoc], field: &Field) -> CellState {
        let (live, _): (Vec<_>, Vec<_>) = neighbors
            .iter()
            .map(|c| field.cell_state(c))
            .partition(|x| x.is_live());
        match self {
            CellState::Live => match live.len() {
                0..=1 => CellState::Dead,
                2..=3 => CellState::Live,
                _ => CellState::Dead,
            },
            CellState::Dead => match live.len() {
                3 => CellState::Live,
                _ => CellState::Dead,
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct GridLoc {
    x: i16,
    y: i16,
}

impl GridLoc {
    pub fn new(x: i16, y: i16) -> GridLoc {
        GridLoc {
            x, y
        }
    }
    /// Calculate a value between 0 and SIZE-1 for GridLoc
    fn bucket(&self) -> usize {
        let size = SIZE as i16;
        ((self.x.wrapping_add(self.y) % size).wrapping_add(size) % size) as usize
    }
}

pub struct Field {
    field: Box<[Vec<(GridLoc, CellState)>; SIZE]>,
    size: usize,
}

impl Field {
    pub fn new() -> Field {
        Self::default()
    }

    pub fn insert(&mut self, pos: GridLoc, state: CellState) {
        let b = pos.bucket();
        let bucket = &mut self.field[b];
        for (i, (loc, _)) in bucket.iter().enumerate() {
            if loc == &pos {
                // If the key is in the Vec, update the tuple by removing it and then pushing the new key value
                bucket.swap_remove(i);
                bucket.push((pos, state));
                return;
            }
        }
        // We didn't already find it, so we have to push it
        self.size += 1;
        bucket.push((pos, state));
    }

    pub fn set<I: Iterator<Item = (GridLoc, CellState)>>(&mut self, coords: I) {
        for (c, s) in coords {
            self.insert(c, s);
        }
    }

    pub fn remove(&mut self, x: &GridLoc) -> Option<CellState> {
        let b = x.bucket();
        let bucket = &mut self.field[b];
        for (i, (loc, s)) in bucket.iter().enumerate() {
            if loc == x {
                self.size -= 1;
                return Some(bucket.swap_remove(i).1);
            }
        }
        None
    }

    pub fn contains(&self, x: &GridLoc) -> bool {
        let b = x.bucket();
        let bucket = &self.field[b];
        for (loc, _) in bucket.iter() {
            if loc == x {
                // It's already in the bucket, so we're done
                return true;
            }
        }
        false
    }

    pub fn cell_state(&self, coord: &GridLoc) -> CellState {
        if self.contains(coord) {
            CellState::Live
        } else {
            CellState::Dead
        }
    }

    pub fn cell_states<'a, I: Iterator<Item = &'a GridLoc>>(
        &self,
        coords: I,
    ) -> Vec<(GridLoc, CellState)> {
        coords.map(|c| (*c, self.cell_state(c))).collect()
    }

    fn area(&self, coord: GridLoc, radius: i16) -> Vec<GridLoc> {
        let mut cs = Vec::new();
        for i in -radius..=radius {
            for j in -radius..=radius {
                cs.push(GridLoc {
                    x: i + coord.x,
                    y: j + coord.y,
                });
            }
        }
        cs
    }

    fn neighbors(&self, coord: GridLoc) -> Vec<GridLoc> {
        let mut cs = Vec::with_capacity(8);
        for i in -1..=1 {
            for j in -1..=1 {
                let c = GridLoc {
                    x: i + coord.x,
                    y: j + coord.y,
                };
                if c != coord {
                    cs.push(c);
                }
            }
        }
        cs
    }

    fn next(&mut self) {
        // coordinates to process
        let mut to_process = self.iter().fold(Vec::new(), |mut acc, x| {
            acc.extend(self.area(x.0, 1));
            acc
        });
        to_process.sort();
        to_process.dedup();

        // Remove currently Dead cells
        // For each coordinate, figure out what to do at that coordinate
        let mut new_live = Vec::new();
        let mut new_dead = Vec::new();
        for (c, current) in self.cell_states(to_process.iter()) {
            let next = current.next(&self.neighbors(c), &self);
            match (current, next) {
                (CellState::Dead, CellState::Live) => new_live.push(c),
                (CellState::Live, CellState::Dead) => new_dead.push(c),
                _ => (),
            }
        }
        // Replace the values
        for live in new_live {
            self.insert(live, CellState::Live);
        }
        for dead in new_dead {
            let _ = self.remove(&dead);
        }
    }

    pub fn iter(&'_ self) -> FieldIterator<'_> {
        FieldIterator {
            bucket_idx: 0,
            inner_idx: 0,
            field: self,
        }
    }
}

impl Default for Field {
    fn default() -> Self {
        const fn empty_vec() -> Vec<(GridLoc, CellState)> {
            Vec::new()
        }
        const EMPTY: Vec<(GridLoc, CellState)> = empty_vec();

        Field {
            field: Box::new([EMPTY; SIZE]),
            size: 0,
        }
    }
}

pub struct FieldIterator<'a> {
    bucket_idx: usize,
    inner_idx: usize,
    field: &'a Field,
}

impl<'a> FieldIterator<'a> {
    fn new(field: &'a Field) -> FieldIterator {
        FieldIterator {
            bucket_idx: 0,
            inner_idx: 0,
            field,
        }
    }
}

impl<'a> Iterator for FieldIterator<'a> {
    type Item = &'a (GridLoc, CellState);

    /// Iterate through all elements
    /// We only advance `outer`` if the bucket is empty, or we've reached the end of the current bucket
    fn next(&mut self) -> Option<Self::Item> {
        while self.bucket_idx < SIZE {
            // get the bucket of the current bucket index
            let bucket = &self.field.field[self.bucket_idx];
            // get the element at the current inner index
            match bucket.get(self.inner_idx) {
                Some(c) => {
                    // We found something so increment inner for the next loop and return Some
                    self.inner_idx += 1;
                    return Some(c);
                }
                None => {
                    // There was no element at the current inner_idx, so,
                    // set inner to 0 for when we go to the next bucket
                    // increment bucket_idx to go to the next bucket
                    // and start the loop again
                    self.inner_idx = 0;
                    self.bucket_idx += 1;
                }
            };
        }
        // Now we've found all the elements and bucket_idx breaks us out of the loop
        // so we return None
        None
    }
}

pub struct Life {
    field: Field,
    cell_size: u32,
    live_style: PrimitiveStyle<color::Rgb3>,
    bg_color: color::Rgb3,
    height: usize,
    width: usize,
    frame: u32,
    generation: u32,
    frames_per_generation: u32,
}

impl Life {
    pub fn new(field: Field) -> Life {
        Life {
            field,
            cell_size: 5,
            bg_color: Rgb3::new(1, 1, 1),
            height: HEIGHT,
            width: WIDTH,
            frame: 0,
            generation: 0,
            frames_per_generation: 35,
            live_style: PrimitiveStyleBuilder::new()
                .stroke_color(Rgb3::new(3, 7, 4))
                .fill_color(Rgb3::new(3, 7, 4))
                .build(),
        }
    }

    pub fn render<D>(&self, display: &mut D)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        // clear
        for y in 0..self.height - 100 {
            for x in 0..self.width {
                video::set_pixel(x, y, self.bg_color.to_byte())
            }
        }

        for (cell, _) in self.field.iter() {
            let top_left = (
                (cell.x * self.cell_size as i16),
                self.height as i16 - cell.y * (self.cell_size as i16) - 1,
            );
            let rect = Rectangle::new(
                Point::new(top_left.0 as i32, top_left.1 as i32),
                Size::new(self.cell_size, self.cell_size),
            );
            let _ = rect.draw_styled(&self.live_style, display);
        }
    }

    pub fn update(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        if self.frame > self.frames_per_generation {
            self.frame = 0;
            self.generation = self.generation.wrapping_add(1);
            self.field.next();
        }
    }

    pub fn update_and_render<D>(&mut self, display: &mut D)
    where
        D: DrawTarget<Color = Rgb3>, {

        self.frame = self.frame.wrapping_add(1);
        if self.frame > self.frames_per_generation {
            self.frame = 0;
            self.generation = self.generation.wrapping_add(1);
            self.render(display);
            self.field.next();
        }
    }
}
