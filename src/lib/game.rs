use crate::{
    lib::{gen_mines, position::Position},
    COLS, ROWS,
};
use std::{collections::HashMap, time::Instant};

pub type FieldMap = HashMap<Position, Field>;

fn gen() -> (Vec<Position>, FieldMap) {
    let mines = gen_mines();

    let mut field = HashMap::new();

    for x in 0..*ROWS {
        for y in 0..*COLS {
            let pos = Position(x, y);
            let is_mine = mines.contains(&pos);
            field.insert(
                pos,
                Field {
                    is_mine,
                    is_clicked: false,
                    is_flagged: false,
                    mines_around: around(&pos, &mines),
                },
            );
        }
    }

    (mines, field)
}

#[derive(Debug, Copy, Clone)]
pub struct Field {
    pub is_mine: bool,
    pub is_clicked: bool,
    pub is_flagged: bool,
    pub mines_around: u16,
}

impl Field {
    pub fn mines_around_class_name(&self) -> String {
        format!(
            "btn_close{}",
            match self.mines_around {
                1 => "_one",
                2 => "_two",
                3 => "_three",
                4 => "_four",
                5 => "_five",
                6 => "_six",
                7 => "_seven",
                8 => "_eight",
                _ => "",
            }
        )
        .to_string()
    }
}

pub struct Game {
    pub mines: Vec<Position>,
    pub field: FieldMap,
    pub active: bool,
    pub ended: bool,
    pub time: Instant,
}

impl Game {
    pub fn new() -> Self {
        let (mines, field) = gen();

        Self {
            mines,
            field,
            active: false,
            ended: false,
            time: Instant::now(),
        }
    }

    pub fn start_timer(&mut self) {
        self.active = true;
        self.time = Instant::now();
    }

    pub fn new_mines(&mut self) {
        let (mines, field) = gen();

        self.mines = mines;
        self.field = field;
        self.active = false;
        self.ended = false;
        self.time = Instant::now();
    }
}

fn around(pos: &Position, mines: &Vec<Position>) -> u16 {
    let (x, y) = (pos.0, pos.1);
    let mut points = vec![(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)];

    let (pos_x, pos_y) = (x > 0, y > 0);

    if pos_y {
        if pos_x {
            points.push((x - 1, y - 1));
        }
        points.push((x, y - 1));
        points.push((x + 1, y - 1));
    }

    if pos_x {
        points.push((x - 1, y));
        points.push((x - 1, y + 1));
    }

    points.iter().fold(0, |total, (x, y)| {
        if mines.contains(&Position(*x, *y)) {
            total + 1
        } else {
            total
        }
    })
}
