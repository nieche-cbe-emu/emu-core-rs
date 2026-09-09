
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Touch {
    Down,
    Move,
    Up,
}

#[derive(Clone, Debug)]
pub enum Event {
    Audio(String),
    Exit,
    Log(String),
}

pub struct Session {
    n: u64,
}

impl Session {
    pub fn open(_path: &str) -> Result<Session, String> {
        Ok(Session { n: 0 })
    }
    pub fn boot(&mut self) -> bool {
        true
    }
    pub fn stop(&mut self) {}
    pub fn set_keys(&mut self, _mask: u32) {}
    pub fn set_touch(&mut self, _x: i32, _y: i32, _s: Touch) {}
    pub fn soft_key(&mut self, _side: &str, _pressed: bool) {}
    pub fn size(&self) -> (u32, u32) {
        (240, 400)
    }
    pub fn take_events(&mut self) -> Vec<Event> {
        Vec::new()
    }
    pub fn step(&mut self) -> Vec<u8> {
        self.n += 1;
        let (w, h) = self.size();
        let mut v = Vec::with_capacity((w * h * 2) as usize);
        for y in 0..h {
            for x in 0..w {
                let c: u16 = (((x * 31 / w) as u16) << 11)
                    | ((((y + self.n as u32) % 64) as u16) << 5)
                    | 15;
                v.extend_from_slice(&c.to_le_bytes());
            }
        }
        v
    }
}
