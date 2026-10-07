
pub const STOPPED: u32 = 0;
pub const PLAYING: u32 = 1;
pub const PAUSED: u32 = 2;

#[derive(Debug, Default)]
pub struct Audio {
    pub state: u32,
    pub volume: u32,
    pub ends_at: u64,
    pub looping: bool,

    pub outdir: Option<std::path::PathBuf>,

    pub dumped: std::collections::HashMap<(String, usize), String>,

    pub events: Vec<String>,
}

const MAGIC: &[(&[u8], &str)] = &[
    (b"MThd", "mid"),
    (b"RIFF", "wav"),
    (b"#!AMR", "amr"),
    (b"ID3", "mp3"),
    (b"\xff\xfb", "mp3"),
    (b"\xff\xf3", "mp3"),
    (b"\xff\xe3", "mp3"),
    (b"\xff\xf2", "mp3"),
];

pub fn sniff(d: &[u8]) -> &'static str {
    for (m, ext) in MAGIC {
        if d.starts_with(m) {
            return ext;
        }
    }
    "bin"
}

const MP3_BR: [[[u32; 15]; 3]; 2] = [
    [
        [0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448],
        [0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384],
        [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320],
    ],
    [
        [0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256],
        [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160],
        [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160],
    ],
];

pub fn mp3_duration_ms(d: &[u8]) -> u64 {
    let n = d.len();
    let mut o = 0usize;
    if n >= 10 && d.starts_with(b"ID3") {

        o = 10 + (((d[6] & 0x7F) as usize) << 21
            | ((d[7] & 0x7F) as usize) << 14
            | ((d[8] & 0x7F) as usize) << 7
            | (d[9] & 0x7F) as usize);
    }
    let mut i = o;
    while i + 4 <= n {
        if d[i] == 0xFF && d[i + 1] & 0xE0 == 0xE0 {
            let ver = if (d[i + 1] >> 3) & 1 == 1 { 1usize } else { 2usize };
            let layer = 4usize - ((d[i + 1] >> 1) & 3) as usize;
            let bri = ((d[i + 2] >> 4) & 0x0F) as usize;
            let sri = (d[i + 2] >> 2) & 0x03;
            if (1..=3).contains(&layer) && bri != 0 && bri != 0x0F && sri != 3 {
                let kbps = MP3_BR[ver - 1][layer - 1][bri] as u64;
                if kbps > 0 {
                    return (((n - i) as u64) * 8 / kbps).max(100);
                }
            }
        }
        i += 1;
    }
    ((n as u64) * 8 / 32).max(500)
}

pub fn wav_duration_ms(d: &[u8]) -> u64 {
    let n = d.len();
    if n < 44 || !d.starts_with(b"RIFF") || &d[8..12] != b"WAVE" {
        return ((n as u64) / 8).max(500);
    }
    let (mut o, mut rate, mut data) = (12usize, 0u64, 0u64);
    while o + 8 <= n {
        let tag = &d[o..o + 4];
        let ln = u32::from_le_bytes([d[o + 4], d[o + 5], d[o + 6], d[o + 7]]) as usize;
        if tag == b"fmt " && ln >= 16 && o + 20 <= n {
            rate = u32::from_le_bytes([d[o + 16], d[o + 17], d[o + 18], d[o + 19]]) as u64;
        } else if tag == b"data" {
            data = ln.min(n - o - 8) as u64;
            break;
        }
        o += 8 + ln + (ln & 1);
    }
    if rate > 0 && data > 0 {
        return (data * 1000 / rate).max(100);
    }
    ((n as u64) / 8).max(500)
}

pub fn midi_duration_ms(d: &[u8]) -> u64 {
    if !d.starts_with(b"MThd") || d.len() < 14 {
        return 1000;
    }
    let div = i16::from_be_bytes([d[12], d[13]]) as i64;
    let mut ticks_total: u64 = 0;
    let mut tempo: u64 = 500_000;
    let mut o = 14usize;
    while o + 8 <= d.len() {
        let tag = &d[o..o + 4];
        let ln = u32::from_be_bytes([d[o + 4], d[o + 5], d[o + 6], d[o + 7]]) as usize;
        let end = (o + 8 + ln).min(d.len());
        let body = &d[(o + 8).min(d.len())..end];
        if tag == b"MTrk" {
            let (mut t, mut p, mut run) = (0u64, 0usize, 0u8);
            while p < body.len() {
                let mut dt: u64 = 0;
                while p < body.len() {
                    let b = body[p];
                    p += 1;
                    dt = (dt << 7) | (b & 0x7F) as u64;
                    if b & 0x80 == 0 {
                        break;
                    }
                }
                t += dt;
                if p >= body.len() {
                    break;
                }
                let mut st = body[p];
                if st < 0x80 {
                    st = run;
                } else {
                    p += 1;
                    run = st;
                }
                if st == 0xFF {
                    if p >= body.len() {
                        break;
                    }
                    let mt = body[p];
                    p += 1;
                    let mut ln2: usize = 0;
                    while p < body.len() {
                        let b = body[p];
                        p += 1;
                        ln2 = (ln2 << 7) | (b & 0x7F) as usize;
                        if b & 0x80 == 0 {
                            break;
                        }
                    }
                    if mt == 0x51 && ln2 == 3 && p + 3 <= body.len() {
                        tempo = ((body[p] as u64) << 16)
                            | ((body[p + 1] as u64) << 8)
                            | body[p + 2] as u64;
                    }
                    p += ln2;
                } else if matches!(st & 0xF0, 0xC0 | 0xD0) {
                    p += 1;
                } else if st & 0xF0 == 0xF0 {

                } else {
                    p += 2;
                }
            }
            ticks_total = ticks_total.max(t);
        }
        o += 8 + ln;
    }
    if div > 0 && ticks_total > 0 {
        return ticks_total * tempo / div as u64 / 1000;
    }
    1000
}

impl Audio {

    fn dump(&mut self, data: &[u8], name: &str) -> Option<String> {
        let dir = self.outdir.clone()?;
        let key = (name.to_string(), data.len());
        if let Some(p) = self.dumped.get(&key) {
            return Some(p.clone());
        }
        std::fs::create_dir_all(&dir).ok()?;
        let ext = sniff(data);
        let base = name
            .to_lowercase()
            .ends_with(&format!(".{ext}"))
            .then(|| &name[..name.len() - ext.len() - 1])
            .unwrap_or(name);

        let safe: String = base
            .chars()
            .map(|c| if c == '/' || c == '\\' { '_' } else { c })
            .collect();
        let path = dir.join(format!("{safe}.{ext}"));
        std::fs::write(&path, data).ok()?;
        let p = path.to_string_lossy().to_string();
        self.dumped.insert(key, p.clone());
        Some(p)
    }

    pub fn play_data(&mut self, data: &[u8], looping: bool, now: u64, name: &str) -> u32 {
        if data.is_empty() {
            self.state = STOPPED;
            return 0;
        }
        let dur = match sniff(data) {
            "mid" => midi_duration_ms(data),
            "mp3" => mp3_duration_ms(data),
            "wav" => wav_duration_ms(data),
            _ => (data.len() as u64 / 8).max(500),
        };
        self.looping = looping;
        self.state = PLAYING;
        self.ends_at = now + dur;
        let kind = sniff(data);
        let path = self.dump(data, name);
        self.events.push(format!(

            "{{\"op\":\"play\",\"path\":{},\"loop\":{},\"ext\":\"{}\",\"name\":{}}}",
            match &path {
                Some(p) => crate::session::json_str(p),
                None => "null".to_string(),
            },
            looping,
            kind,
            crate::session::json_str(name)
        ));
        1
    }

    pub fn tick(&mut self, now: u64) {
        if self.state == PLAYING && now >= self.ends_at {
            if self.looping {
                let d = self.ends_at.saturating_sub(now);
                self.ends_at = now + if d == 0 { 1000 } else { d };
            } else {
                self.state = STOPPED;
                self.events.push("{\"op\":\"stop\"}".to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mp3(b1: u8, b2: u8, total: usize) -> Vec<u8> {
        let mut v = vec![0xFF, b1, b2, 0x00];
        v.resize(total, 0);
        v
    }

    fn id3(size: usize, inner: &[u8]) -> Vec<u8> {
        let n = size;
        let mut v = vec![b'I', b'D', b'3', 3, 0, 0];
        v.extend_from_slice(&[
            ((n >> 21) & 0x7F) as u8,
            ((n >> 14) & 0x7F) as u8,
            ((n >> 7) & 0x7F) as u8,
            (n & 0x7F) as u8,
        ]);
        v.resize(10 + n, 0);
        v.extend_from_slice(inner);
        v
    }

    fn wav(byte_rate: u32, data_len: usize) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&8000u32.to_le_bytes());
        v.extend_from_slice(&byte_rate.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&8u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&(data_len as u32).to_le_bytes());
        v.resize(44 + data_len, 0);
        v
    }

    #[test]
    fn mp3_duration_ms_cases() {
        assert_eq!(mp3_duration_ms(&mp3(0xFB, 0x90, 29227)), 29227 * 8 / 128);
        assert_eq!(mp3_duration_ms(&mp3(0xF3, 0x10, 35428)), 35428 * 8 / 8);
        assert_eq!(mp3_duration_ms(&mp3(0xFB, 0x10, 91869)), 91869 * 8 / 32);
        assert_eq!(mp3_duration_ms(&mp3(0xFB, 0x30, 91869)), 91869 * 8 / 48);
        assert_eq!(mp3_duration_ms(&id3(1024, &mp3(0xFB, 0x90, 4096))), 4096 * 8 / 128);
        assert_eq!(mp3_duration_ms(&[0xFF]), 500);
        assert_eq!(mp3_duration_ms(&[0x11; 8000]), 8000 * 8 / 32);
    }

    #[test]
    fn wav_duration_ms_cases() {
        assert_eq!(wav_duration_ms(&wav(32000, 64000)), 2000);
        let mut bad = Vec::new();
        bad.extend_from_slice(b"RIFF\x00\x00\x00\x00WAVE");
        bad.resize(76, 0);
        assert_eq!(wav_duration_ms(&bad), 500);
    }
}
