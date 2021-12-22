use crate::util::InitOnce;
use libsys::mem::read_le32;
use crate::dev::display::FramebufferInfo;

static FONT_DATA: &[u8] = include_bytes!("../../etc/default8x16.psfu");
static FONT: InitOnce<Font> = InitOnce::new();

pub struct Font {
    char_width: usize,
    char_height: usize,
    bytes_per_glyph: usize,
    data: &'static [u8],
}

impl Font {
    pub fn draw(&self, fb: &FramebufferInfo, bx: usize, by: usize, ch: char, fg: u32, bg: u32) {
        if ch >= ' ' && ch < '\x7B' {
            let char_data = &self.data[ch as usize * self.bytes_per_glyph..];

            for iy in 0..self.char_height {
                for ix in 0..self.char_width {
                    let cx = self.char_width - ix - 1;
                    let ptr = fb.virt_base + (ix + bx + (iy + by) * fb.width) * 4;
                    let value = if char_data[iy + (cx) / 8] & (1 << (cx & 0x7)) != 0 {
                        fg
                    } else {
                        bg
                    };
                    unsafe { core::ptr::write_volatile(ptr as *mut u32, value) }
                }
            }
        }
    }
}

pub fn init() {
    assert_eq!(read_le32(&FONT_DATA[..]), 0x864ab572);

    FONT.init(Font {
        char_width: read_le32(&FONT_DATA[28..]) as usize,
        char_height: read_le32(&FONT_DATA[24..]) as usize,
        bytes_per_glyph: read_le32(&FONT_DATA[20..]) as usize,
        data: &FONT_DATA[32..]
    });
}

pub fn get() -> &'static Font {
    FONT.get()
}
