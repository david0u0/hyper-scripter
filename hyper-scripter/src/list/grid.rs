use term_grid::{Cell, Direction, Filling, Grid as LibGrid, GridOptions};

pub struct Grid {
    capacity: usize,
    grid: LibGrid,
}

impl Grid {
    pub fn new(capacity: usize) -> Self {
        let mut grid = LibGrid::new(GridOptions {
            direction: Direction::TopToBottom,
            filling: Filling::Spaces(2),
        });
        grid.reserve(capacity);
        Self { grid, capacity }
    }
    pub fn add(&mut self, contents: String, width: usize) {
        let cell = Cell { contents, width };
        self.grid.add(cell);
    }
    pub fn clear(&mut self) {
        *self = Grid::new(self.capacity);
    }
    pub fn fit_into_screen<'a>(&'a mut self) -> impl std::fmt::Display + 'a {
        let width = console::Term::stdout().size().1 as usize;
        if let Some(grid_display) = self.grid.fit_into_width(width) {
            grid_display
        } else {
            self.grid.fit_into_columns(1)
        }
    }
}
