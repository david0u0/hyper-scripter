use term_grid::{Cell, Direction, Filling, Grid as LibGrid, GridOptions};

pub struct Grid {
    capacity: usize,
    grid: LibGrid,
    term_width: Option<usize>,
}

impl Grid {
    pub fn new(capacity: usize) -> Self {
        let mut grid = LibGrid::new(GridOptions {
            direction: Direction::TopToBottom,
            filling: Filling::Spaces(2),
        });
        grid.reserve(capacity);
        Self {
            grid,
            capacity,
            term_width: None,
        }
    }
    pub fn add(&mut self, contents: String, width: usize) {
        let cell = Cell { contents, width };
        self.grid.add(cell);
    }
    pub fn clear(&mut self) {
        *self = Grid::new(self.capacity);
    }
    pub fn fit_into_screen<'a>(&'a mut self) -> impl std::fmt::Display + 'a {
        if self.term_width.is_none() {
            let w = console::Term::stdout().size_checked().map_or(0, |s| s.1);
            self.term_width = Some(w as usize);
        }

        let width = self.term_width.unwrap();
        if width > 0 {
            if let Some(grid_display) = self.grid.fit_into_width(width) {
                return grid_display;
            }
        }
        self.grid.fit_into_columns(1)
    }
}
