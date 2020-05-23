#![allow(dead_code)]

use std::sync::Once;
use std::fmt;
use std::default;
use std::collections::HashMap;
use std::cell::Cell;
use std::ops;

static ENCODE_TABLE: &[u8] = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ?!".as_bytes();
static mut DECODE_TABLE: [u8; 256] = [0; 256];
static DECODE_TABLE_INIT: Once = Once::new();

fn decode_base64(ch: char) -> u8 {
    unsafe {
        DECODE_TABLE_INIT.call_once(|| {
            for (i, byte) in ENCODE_TABLE.iter().enumerate() {
                DECODE_TABLE[*byte as usize] = i as u8;
            }
        });
        DECODE_TABLE[ch as usize]
    }
}

fn encode_base64(int: u8) -> char {
    if int as usize >= ENCODE_TABLE.len() {
        panic!("out of range");
    }
    ENCODE_TABLE[int as usize] as char
}

//

#[derive(Copy, Clone)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    fn zero() -> Self {
        Self { x: 0, y: 0 }
    }

    fn translate(&self, x: i32, y: i32) -> Self {
        Self {
            x: self.x + x,
            y: self.y + y,
        }
    }
}

impl ops::Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl ops::Sub for Point {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

//

struct Matrix<T> {
    width: usize,
    height: usize,
    data: Vec<Vec<T>>
}

impl<T: default::Default + Clone> Matrix<T> {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![vec![Default::default(); width]; height],
        }
    }

    fn ref_idx(&self, pt: Point) -> &T {
        &self.data[pt.y as usize][pt.x as usize]
    }

    fn in_bounds(&self, pt: Point) -> bool {
        pt.x >= 0 && pt.y >= 0
            && pt.x < self.width as i32
            && pt.y < self.height as i32
    }

    fn indexed_iter(&self) -> MatrixIterator<T> {
        MatrixIterator {
            matr: self,
            at: Point::new(-1, 0),
        }
    }
}

struct MatrixIterator<'a, T> {
    matr: &'a Matrix<T>,
    at: Point,
}

impl<'a, T: default::Default + Clone> Iterator for MatrixIterator<'a, T> {
    type Item = (Point, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.at.x += 1;

        if self.at.x >= self.matr.width as i32 {
            self.at.y += 1;
            self.at.x = 0;
        }

        if self.at.y < self.matr.height as i32 {
            Some((self.at, self.matr.ref_idx(self.at)))
        } else {
            None
        }
    }
}

//

#[derive(Clone)]
struct Slot {
    operator: Cell<char>,
    lock: Cell<bool>
}

impl Slot {
    fn is_clear(&self) -> bool {
        self.operator.get() == '\0'
    }

    fn explode(&self) {
        self.operator.set('*');
    }

    fn clear(&self) {
        self.operator.set('\0');
    }
}

impl default::Default for Slot {
    fn default() -> Self {
        Self {
            operator: Cell::new('\0'),
            lock: Cell::new(false),
        }
    }
}

impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let op = self.operator.get();
        let lk = self.lock.get();

        let ch = if op == '\0' { '.' } else { op };

        if lk {
            write!(f, "[{}]", ch)
        } else {
            write!(f, " {} ", ch)
        }
    }
}

//

struct Field {
    slots: Matrix<Slot>
}

impl Field {
    fn new(width: usize, height: usize) -> Self {
        Self {
            slots: Matrix::new(width, height)
        }
    }

    fn unlock_all(&mut self) {
        for (_pt, slot) in self.slots.indexed_iter() {
            slot.lock.set(false);
        }
    }

    fn ref_slot(&self, pt: Point) -> &Slot {
        self.slots.ref_idx(pt)
    }

    fn point_in_bounds(&self, pt: Point) -> bool {
        self.slots.in_bounds(pt)
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (pt, slot) in self.slots.indexed_iter() {
            write!(f, "{}", slot)?;
            if pt.x + 1 == self.slots.width as i32 {
                writeln!(f, "")?;
            }
        }
        write!(f, "")
    }
}

//

struct Opdef {
    long_name: String,
    operator: char,
    callback: fn(&Context) -> (),
}

struct OpdefTable(HashMap<char, Opdef>);

impl OpdefTable {
    fn new() -> OpdefTable {
        OpdefTable(HashMap::new())
    }

    fn add(&mut self, opd: Opdef) {
        self.0.insert(opd.operator, opd);
    }

    fn find(&self, ch: char) -> Option<&Opdef> {
        self.0.get(&ch)
    }
}

static NORTH: Point = Point { x:  0, y: -1 };
static SOUTH: Point = Point { x:  0, y:  1 };
static EAST: Point  = Point { x:  1, y:  0 };
static WEST: Point  = Point { x: -1, y:  0 };

impl default::Default for OpdefTable {
    fn default() -> Self {
        let mut ret = OpdefTable::new();
        ret.add(Opdef {
            long_name: "bang".to_string(),
            operator: '*',
            callback: | ctx: &Context | {
                let ref current_slot = ctx.field.ref_slot(ctx.curr_point);
                current_slot.clear();
                current_slot.lock.set(true);
            }
        });
        ret.add(Opdef {
            long_name: "east".to_string(),
            operator: 'E',
            callback: | ctx: &Context | {
                move_direction(ctx, EAST);
            }
        });
        ret.add(Opdef {
            long_name: "west".to_string(),
            operator: 'W',
            callback: | ctx: &Context | {
                move_direction(ctx, WEST);
            }
        });
        ret.add(Opdef {
            long_name: "north".to_string(),
            operator: 'N',
            callback: | ctx: &Context | {
                move_direction(ctx, NORTH);
            }
        });
        ret.add(Opdef {
            long_name: "south".to_string(),
            operator: 'S',
            callback: | ctx: &Context | {
                move_direction(ctx, SOUTH);
            }
        });
        ret.add(Opdef {
            long_name: "halt".to_string(),
            operator: 'H',
            callback: | ctx: &Context | {
                let next = ctx.curr_point + SOUTH;
                if ctx.field.point_in_bounds(next) {
                    ctx.field.ref_slot(next).lock.set(true);
                }
            }
        });
        ret
    }
}

//

struct Context {
    opdef_table: OpdefTable,
    field: Field,
    curr_point: Point,
    frame_ct: u32,
}

impl Context {
    fn new(opdef_table: OpdefTable, field: Field) -> Context {
        Context {
            opdef_table,
            field,
            curr_point: Point::zero(),
            frame_ct: 0,
        }
    }

    fn process(&mut self) {
        self.field.unlock_all();

        for (pt, slot) in self.field.slots.indexed_iter() {
            self.curr_point = pt;

            let op = slot.operator.get();
            let lk = slot.lock.get();

            if !lk && (op != '\0') {
                let ref opd = self.opdef_table.find(op)
                                  .expect("operator not found");
                (opd.callback)(self);
            }

            self.frame_ct += 1;
        }
    }
}

//

fn move_direction(ctx: &Context, translate: Point) {
    let next = ctx.curr_point + translate;
    let ref current_slot = ctx.field.ref_slot(ctx.curr_point);

    if !ctx.field.point_in_bounds(next) ||
       !ctx.field.ref_slot(next).is_clear() {
        current_slot.explode();
        current_slot.lock.set(true);
    } else {
        let next_slot = ctx.field.ref_slot(next);
        next_slot.operator.set(current_slot.operator.get());
        next_slot.lock.set(true);
        current_slot.clear();
        current_slot.lock.set(true);
    }
}

//

fn main() {
    let opdt: OpdefTable = Default::default();
    let field = Field::new(10, 15);
    let mut ctx = Context::new(opdt, field);

    ctx.field.ref_slot(Point::new(0, 0)).operator.set('*');
    ctx.field.ref_slot(Point::new(3, 3)).operator.set('E');
    ctx.field.ref_slot(Point::new(3, 5)).operator.set('E');
    ctx.field.ref_slot(Point::new(3, 4)).operator.set('W');
    ctx.field.ref_slot(Point::new(6, 4)).operator.set('H');

    println!("{}", ctx.field);
    ctx.process();
    println!("{}", ctx.field);
    ctx.process();
    println!("{}", ctx.field);
    ctx.process();
    println!("{}", ctx.field);
    ctx.process();
    println!("{}", ctx.field);
}
