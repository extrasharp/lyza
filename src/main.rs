#![allow(dead_code)]

use std::sync::Once;
use std::fmt;
use std::default;
use std::collections::HashMap;
use std::cell::Cell;

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

#[derive(Clone)]
struct Slot {
    operator: Cell<char>,
    lock: Cell<bool>
}

impl Slot {
    fn new() -> Slot {
        Slot {
            operator: Cell::new('\0'),
            lock: Cell::new(false),
        }
    }

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
    width: usize,
    height: usize,
    slots: Vec<Vec<Slot>>,
}

impl Field {
    fn new(width: usize, height: usize) -> Field {
        Field {
            width,
            height,
            slots: vec![vec![Slot::new(); width]; height],
        }
    }

    fn unlock_all(&mut self) {
        for row in self.slots.iter() {
            for slot in row {
                slot.lock.set(false);
            }
        }
    }

    fn ref_slot(&self, x: usize, y: usize) -> &Slot {
        &self.slots[x][y]
    }

    fn point_in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >=0 && x < self.width as i32 && y < self.height as i32
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for row in self.slots.iter() {
            for slot in row {
                write!(f, "{}", slot)?;
            }
            writeln!(f, "")?;
        }
        write!(f, "")
    }
}

//

struct Opdef {
    long_name: String,
    ch: char,
    callback: fn(&Context) -> (),
}

struct OpdefTable(HashMap<char, Opdef>);

impl OpdefTable {
    fn new() -> OpdefTable {
        OpdefTable(HashMap::new())
    }


    fn add(&mut self, opd: Opdef) {
        self.0.insert(opd.ch, opd);
    }

    fn find(&self, ch: char) -> Option<&Opdef> {
        self.0.get(&ch)
    }

}

impl default::Default for OpdefTable {
    fn default() -> Self {
        let mut ret = OpdefTable::new();
        ret.add(Opdef {
            long_name: "bang".to_string(),
            ch: '*',
            callback: | ctx: &Context | {
                clear(ctx, ctx.curr_x, ctx.curr_y);
            }
        });
        ret.add(Opdef {
            long_name: "east".to_string(),
            ch: 'E',
            callback: | ctx: &Context | {
                move_direction(ctx, 0, 1);
            }
        });
        ret.add(Opdef {
            long_name: "west".to_string(),
            ch: 'W',
            callback: | ctx: &Context | {
                move_direction(ctx, 0, -1);
            }
        });
        ret
    }
}

//

struct Context {
    opdef_table: OpdefTable,
    field: Field,
    curr_x: usize,
    curr_y: usize,
    frame_ct: u32,
}

impl Context {
    fn new(opdef_table: OpdefTable, field: Field) -> Context {
        Context { opdef_table, field
                , curr_x: 0
                , curr_y: 0
                , frame_ct: 0
                }
    }

    fn process(&mut self) {
        self.field.unlock_all();

        for (x, row) in self.field.slots.iter().enumerate() {
            for (y, slot) in row.iter().enumerate() {
                self.curr_x = x;
                self.curr_y = y;

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

    fn current_slot(&self) -> &Slot {
        self.field.ref_slot(self.curr_x, self.curr_y)
    }
}

//

fn explode(ctx: &Context, x: usize, y: usize) {
    let ref slot = ctx.field.ref_slot(x, y);
    slot.explode();
    slot.lock.set(true);
}

fn clear(ctx: &Context, x: usize, y: usize) {
    let ref slot = ctx.field.ref_slot(x, y);
    slot.clear();
    slot.lock.set(true);
}

fn move_direction(ctx: &Context, dx: i32, dy: i32) {
    let next_x = ctx.curr_x as i32 + dx;
    let next_y = ctx.curr_y as i32 + dy;

    if !ctx.field.point_in_bounds(next_x, next_y) ||
       !ctx.field.ref_slot(next_x as usize, next_y as usize).is_clear() {
        explode(ctx, ctx.curr_x, ctx.curr_y);
    } else {
        let next_slot = ctx.field.ref_slot(next_x as usize, next_y as usize);
        next_slot.operator.set(ctx.current_slot().operator.get());
        next_slot.lock.set(true);
        clear(ctx, ctx.curr_x, ctx.curr_y);
    }
}

//

fn main() {
    let opdt: OpdefTable = Default::default();
    let field = Field::new(10, 15);
    let mut ctx = Context::new(opdt, field);

    ctx.field.ref_slot(0, 0).operator.set('*');
    ctx.field.ref_slot(3, 3).operator.set('E');
    ctx.field.ref_slot(4, 4).operator.set('W');

    println!("{}", ctx.field);
    ctx.process();
    println!("{}", ctx.field);
    ctx.process();
    println!("{}", ctx.field);
}
