use core::{ffi::{c_char, c_void}, ptr::{null, null_mut}};

use crate::{kernel::{interrupt::send_eoi, io::{inb, KEYBOARD_DATA_PORT, outb, KEYBOARD_CTRL_PORT}, input::{InputEvent, EV_KEY}}, logk, printk};

use super::interrupt::{self, IRQ_KEYBOARD, set_interrupt_mask};
const INV : char = '\0';

static mut KEYBOARD : KeyBoard = KeyBoard::new();
static mut SCAN_CODE_SET2 : ScanCodeSet2 = ScanCodeSet2::new();

pub const KEY_RESERVED : u16 = 0;
pub const KEY_ESC : u16 = 1;
pub const KEY_1 : u16 = 2;
pub const KEY_2 : u16 = 3;
pub const KEY_3 : u16 = 4;
pub const KEY_4 : u16 = 5;
pub const KEY_5 : u16 = 6;
pub const KEY_6 : u16 = 7;
pub const KEY_7 : u16 = 8;
pub const KEY_8 : u16 = 9;
pub const KEY_9 : u16 = 10;
pub const KEY_0 : u16 = 11;
pub const KEY_MINUS : u16 = 12;
pub const KEY_EQUAL : u16 = 13;
pub const KEY_BACKSPACE : u16 = 14;
pub const KEY_TAB : u16 = 15;
pub const KEY_Q : u16 = 16;
pub const KEY_W : u16 = 17;
pub const KEY_E : u16 = 18;
pub const KEY_R : u16 = 19;
pub const KEY_T : u16 = 20;
pub const KEY_Y : u16 = 21;
pub const KEY_U : u16 = 22;
pub const KEY_I : u16 = 23;
pub const KEY_O : u16 = 24;
pub const KEY_P : u16 = 25;
pub const KEY_LEFTBRACE : u16 = 26;
pub const KEY_RIGHTBRACE : u16 = 27;
pub const KEY_ENTER : u16 = 28;
pub const KEY_LEFTCTRL : u16 = 29;
pub const KEY_A	: u16 = 30;
pub const KEY_S : u16 = 31;
pub const KEY_D : u16 = 32;
pub const KEY_F : u16 = 33;
pub const KEY_G : u16 = 34;
pub const KEY_H : u16 = 35;
pub const KEY_J : u16 = 36;
pub const KEY_K : u16 = 37;
pub const KEY_L : u16 = 38;
pub const KEY_SEMICOLON : u16 = 39;
pub const KEY_APOSTROPHE : u16 = 40;
pub const KEY_GRAVE : u16 = 41;
pub const KEY_LEFTSHIFT : u16 = 42;
pub const KEY_BACKSLASH : u16 = 43;
pub const KEY_Z : u16 = 44;
pub const KEY_X : u16 = 45;
pub const KEY_C : u16 = 46;
pub const KEY_V : u16 = 47;
pub const KEY_B : u16 = 48;
pub const KEY_N : u16 = 49;
pub const KEY_M : u16 = 50;
pub const KEY_COMMA : u16 = 51;
pub const KEY_DOT : u16 = 52;
pub const KEY_SLASH : u16 = 53;
pub const KEY_RIGHTSHIFT : u16 = 54;
pub const KEY_KPASTERISK : u16 = 55;
pub const KEY_LEFTALT : u16 = 56;
pub const KEY_SPACE : u16 = 57;
pub const KEY_CAPSLOCK : u16 = 58;
pub const KEY_F1 : u16 = 59;
pub const KEY_F2 : u16 = 60;
pub const KEY_F3 : u16 = 61;
pub const KEY_F4 : u16 = 62;
pub const KEY_F5 : u16 = 63;
pub const KEY_F6 : u16 = 64;
pub const KEY_F7 : u16 = 65;
pub const KEY_F8 : u16 = 66;
pub const KEY_F9 : u16 = 67;
pub const KEY_F10 : u16 = 68;
pub const KEY_NUMLOCK : u16 = 69;
pub const KEY_SCROLLLOCK : u16 = 70;
pub const KEY_KP7 : u16 = 71;
pub const KEY_KP8 : u16 = 72;
pub const KEY_KP9 : u16 = 73;
pub const KEY_KPMINUS : u16 = 74;
pub const KEY_KP4 : u16 = 75;
pub const KEY_KP5 : u16 = 76;
pub const KEY_KP6 : u16 = 77;
pub const KEY_KPPLUS : u16 = 78;
pub const KEY_KP1 : u16 = 79;
pub const KEY_KP2 : u16 = 80;
pub const KEY_KP3 : u16 = 81;
pub const KEY_KP0 : u16 = 82;
pub const KEY_KPDOT : u16 = 83;

pub const KEY_ZENKAKUHANKAKU : u16 = 85;
pub const KEY_102ND : u16 = 86;
pub const KEY_F11 : u16 = 87;
pub const KEY_F12 : u16 = 88;
pub const KEY_RO : u16 = 89;
pub const KEY_KATAKANA : u16 = 90;
pub const KEY_HIRAGANA : u16 = 91;
pub const KEY_HENKAN : u16 = 92;
pub const KEY_KATAKANAHIRAGANA : u16 = 93;
pub const KEY_MUHENKAN : u16 = 94;
pub const KEY_KPJPCOMMA : u16 = 95;
pub const KEY_KPENTER : u16 = 96;
pub const KEY_RIGHTCTRL : u16 = 97;
pub const KEY_KPSLASH : u16 = 98;
pub const KEY_SYSRQ : u16 = 99;
pub const KEY_RIGHTALT : u16 = 100;
pub const KEY_LINEFEED : u16 = 101;
pub const KEY_HOME : u16 = 102;
pub const KEY_UP : u16 = 103;
pub const KEY_PAGEUP : u16 = 104;
pub const KEY_LEFT : u16 = 105;
pub const KEY_RIGHT : u16 = 106;
pub const KEY_END : u16 = 107;
pub const KEY_DOWN : u16 = 108;
pub const KEY_PAGEDOWN : u16 = 109;
pub const KEY_INSERT : u16 = 110;
pub const KEY_DELETE : u16 = 111;
pub const KEY_MACRO : u16 = 112;
pub const KEY_MUTE : u16 = 113;
pub const KEY_VOLUMEDOWN : u16 = 114;
pub const KEY_VOLUMEUP : u16 = 115;
pub const KEY_POWER : u16 = 116;	/* SC System Power Down */
pub const KEY_KPEQUAL : u16 = 117;
pub const KEY_KPPLUSMINUS : u16 = 118;
pub const KEY_PAUSE : u16 = 119;
pub const KEY_SCALE : u16 = 120;	/* AL Compiz Scale (Expose) */

pub const KEY_KPCOMMA : u16 = 121;
pub const KEY_HANGEUL : u16 = 122;
pub const KEY_HANGUEL : u16 = KEY_HANGEUL;
pub const KEY_HANJA : u16 = 123;
pub const KEY_YEN : u16 = 124;
pub const KEY_LEFTMETA : u16 = 125;
pub const KEY_RIGHTMETA : u16 = 126;
pub const KEY_COMPOSE : u16 = 127;

pub const KEY_STOP : u16 = 128;	/* AC Stop */
pub const KEY_AGAIN : u16 = 129;
pub const KEY_PROPS : u16 = 130;	/* AC Properties */
pub const KEY_UNDO : u16 = 131;	/* AC Undo */
pub const KEY_FRONT : u16 = 132;
pub const KEY_COPY : u16 = 133;	/* AC Copy */
pub const KEY_OPEN : u16 = 134;	/* AC Open */
pub const KEY_PASTE : u16 = 135;	/* AC Paste */
pub const KEY_FIND : u16 = 136;	/* AC Search */
pub const KEY_CUT : u16 = 137;	/* AC Cut */
pub const KEY_HELP : u16 = 138;	/* AL Integrated Help Center */
pub const KEY_MENU : u16 = 139;	/* Menu (show menu) */
pub const KEY_CALC : u16 = 140;	/* AL Calculator */
pub const KEY_SETUP : u16 = 141;
pub const KEY_SLEEP : u16 = 142;	/* SC System Sleep */
pub const KEY_WAKEUP : u16 = 143;	/* System Wake Up */
pub const KEY_FILE : u16 = 144;	/* AL Local Machine Browser */
pub const KEY_SENDFILE : u16 = 145;
pub const KEY_DELETEFILE : u16 = 146;
pub const KEY_XFER : u16 = 147;
pub const KEY_PROG1 : u16 = 148;
pub const KEY_PROG2 : u16 = 149;
pub const KEY_WWW : u16 = 150;	/* AL Internet Browser */
pub const KEY_MSDOS : u16 = 151;
pub const KEY_COFFEE : u16 = 152;	/* AL Terminal Lock/Screensaver */
pub const KEY_SCREENLOCK : u16 = KEY_COFFEE;
pub const KEY_ROTATE_DISPLAY : u16 = 153;	/* Display orientation for e.g. tablets */
pub const KEY_DIRECTION : u16 = KEY_ROTATE_DISPLAY;
pub const KEY_CYCLEWINDOWS : u16 = 154;
pub const KEY_MAIL : u16 = 155;
pub const KEY_BOOKMARKS : u16 = 156;	/* AC Bookmarks */
pub const KEY_COMPUTER : u16 = 157;
pub const KEY_BACK : u16 = 158;	/* AC Back */
pub const KEY_FORWARD : u16 = 159;	/* AC Forward */
pub const KEY_CLOSECD : u16 = 160;
pub const KEY_EJECTCD : u16 = 161;
pub const KEY_EJECTCLOSECD : u16 = 162;
pub const KEY_NEXTSONG : u16 = 163;
pub const KEY_PLAYPAUSE : u16 = 164;
pub const KEY_PREVIOUSSONG : u16 = 165;
pub const KEY_STOPCD : u16 = 166;
pub const KEY_RECORD : u16 = 167;
pub const KEY_REWIND : u16 = 168;
pub const KEY_PHONE : u16 = 169;	/* Media Select Telephone */
pub const KEY_ISO : u16 = 170;
pub const KEY_CONFIG : u16 = 171;	/* AL Consumer Control Configuration */
pub const KEY_HOMEPAGE : u16 = 172;	/* AC Home */
pub const KEY_REFRESH : u16 = 173;	/* AC Refresh */
pub const KEY_EXIT : u16 = 174;	/* AC Exit */
pub const KEY_MOVE : u16 = 175;
pub const KEY_EDIT : u16 = 176;
pub const KEY_SCROLLUP : u16 = 177;
pub const KEY_SCROLLDOWN : u16 = 178;
pub const KEY_KPLEFTPAREN : u16 = 179;
pub const KEY_KPRIGHTPAREN : u16 = 180;
pub const KEY_NEW : u16 = 181;	/* AC New */
pub const KEY_REDO : u16 = 182;	/* AC Redo/Repeat */

pub const KEY_F13 : u16 = 183;
pub const KEY_F14 : u16 = 184;
pub const KEY_F15 : u16 = 185;
pub const KEY_F16 : u16 = 186;
pub const KEY_F17 : u16 = 187;
pub const KEY_F18 : u16 = 188;
pub const KEY_F19 : u16 = 189;
pub const KEY_F20 : u16 = 190;
pub const KEY_F21 : u16 = 191;
pub const KEY_F22 : u16 = 192;
pub const KEY_F23 : u16 = 193;
pub const KEY_F24 : u16 = 194;

pub const KEY_PLAYCD : u16 = 200;
pub const KEY_PAUSECD : u16 = 201;
pub const KEY_PROG3 : u16 = 202;
pub const KEY_PROG4 : u16 = 203;
pub const KEY_ALL_APPLICATIONS : u16 = 204;	/* AC Desktop Show All Applications */
pub const KEY_DASHBOARD : u16 = KEY_ALL_APPLICATIONS;
pub const KEY_SUSPEND : u16 = 205;
pub const KEY_CLOSE : u16 = 206;	/* AC Close */
pub const KEY_PLAY : u16 = 207;
pub const KEY_FASTFORWARD : u16 = 208;
pub const KEY_BASSBOOST : u16 = 209;
pub const KEY_PRINT : u16 = 210;	/* AC Print */
pub const KEY_HP : u16 = 211;
pub const KEY_CAMERA : u16 = 212;
pub const KEY_SOUND : u16 = 213;
pub const KEY_QUESTION : u16 = 214;
pub const KEY_EMAIL : u16 = 215;
pub const KEY_CHAT : u16 = 216;
pub const KEY_SEARCH : u16 = 217;
pub const KEY_CONNECT : u16 = 218;
pub const KEY_FINANCE : u16 = 219;	/* AL Checkbook/Finance */
pub const KEY_SPORT : u16 = 220;
pub const KEY_SHOP : u16 = 221;
pub const KEY_ALTERASE : u16 = 222;
pub const KEY_CANCEL : u16 = 223;	/* AC Cancel */
pub const KEY_BRIGHTNESSDOWN : u16 = 224;
pub const KEY_BRIGHTNESSUP : u16 = 225;
pub const KEY_MEDIA : u16 = 226;

pub const KEY_SWITCHVIDEOMODE : u16 = 227;	/* Cycle between available video
         		   outputs (Monitor/LCD/TV-out/etc) */
pub const KEY_KBDILLUMTOGGLE : u16 = 228;
pub const KEY_KBDILLUMDOWN : u16 = 229;
pub const KEY_KBDILLUMUP : u16 = 230;

pub const KEY_SEND : u16 = 231;	/* AC Send */
pub const KEY_REPLY : u16 = 232;	/* AC Reply */
pub const KEY_FORWARDMAIL : u16 = 233;	/* AC Forward Msg */
pub const KEY_SAVE : u16 = 234;	/* AC Save */
pub const KEY_DOCUMENTS : u16 = 235;

pub const KEY_BATTERY : u16 = 236;

pub const KEY_BLUETOOTH : u16 = 237;
pub const KEY_WLAN : u16 = 238;
pub const KEY_UWB : u16 = 239;

pub const KEY_UNKNOWN : u16 = 240;

pub const KEY_VIDEO_NEXT : u16 = 241;	/* drive next video source */
pub const KEY_VIDEO_PREV : u16 = 242;	/* drive previous video source */
pub const KEY_BRIGHTNESS_CYCLE : u16 = 243;	/* brightness up, after max is min */
pub const KEY_BRIGHTNESS_AUTO : u16 = 244;	/* Set Auto Brightness: manual
					  brightness control is off,
					  rely on ambient */
pub const KEY_BRIGHTNESS_ZERO : u16 = KEY_BRIGHTNESS_AUTO;
pub const KEY_DISPLAY_OFF : u16 = 245;	/* display device to off state */

pub const KEY_WWAN : u16 = 246;	/* Wireless WAN (LTE, UMTS, GSM, etc.) */
pub const KEY_WIMAX : u16 = KEY_WWAN;
pub const KEY_RFKILL : u16 = 247;	/* Key that controls all radios */

pub const KEY_MICMUTE : u16 = 248;	/* Mute / unmute the microphone */

static mut KEY_STATUS : [bool; 256] = [false; 256];

pub fn keyboard_init()
{
    unsafe
    {
        KEYBOARD.scan_code_type = ScanCodeType::ScanCOdeSet2;
        // outb(KEYBOARD_DATA_PORT, 0xf0);
        // let mut ack = 0;
        // while ack != 0xFA
        // {
        //     ack = inb(KEYBOARD_DATA_PORT);
        // }
        // outb(KEYBOARD_DATA_PORT, 0x02);
        // while ack != 0xFA
        // {
        //     ack = inb(KEYBOARD_DATA_PORT);
        // }
        SCAN_CODE_SET2.init(&mut KEYBOARD);
    }
}

enum ScanCodeType
{
    None,
    ScanCOdeSet1,
    ScanCOdeSet2,
    ScanCOdeSet3,
}

struct KeyBoard
{
    scan_code_type : ScanCodeType,
    capslock_state : bool,
    scrlock_state : bool,
    numlock_state : bool,
    scan_code_set : *mut c_void
}

impl KeyBoard
{
    const fn new() -> Self
    {
        Self { scan_code_type: ScanCodeType::None, capslock_state: false, scrlock_state: false, numlock_state: false, scan_code_set: null_mut()  }
    }
}

struct ScanCodeSet2<'a>
{
    key_map : [u16; 132],
    ext_code_state : bool,
    break_state : bool,
    keyboard : Option<&'a mut KeyBoard>
}

impl<'a> ScanCodeSet2<'a>
{
    pub fn init(&mut self, keyboard_ref : &'a mut KeyBoard)
    {
        self.keyboard = Some(keyboard_ref);
        outb(KEYBOARD_CTRL_PORT, 0x60);
        outb(KEYBOARD_DATA_PORT, 0b00100001);
        interrupt::regist_irq(keyboard_scan_code_set2_handler as interrupt::HandlerFn, IRQ_KEYBOARD);
        set_interrupt_mask(IRQ_KEYBOARD as u32, true);
    }
    pub const fn new() -> Self
    {
        Self { key_map: [
                /* 0x00 */      KEY_RESERVED,
                /* 0x01 */      KEY_F9,
                /* 0x02 */      KEY_RESERVED,
                /* 0x03 */      KEY_F5,
                /* 0x04 */      KEY_F3,
                /* 0x05 */      KEY_F1,
                /* 0x06 */      KEY_F2,
                /* 0x07 */      KEY_F12,
                /* 0x08 */      KEY_RESERVED,
                /* 0x09 */      KEY_F10,
                /* 0x0A */      KEY_F8,
                /* 0x0B */      KEY_F6,
                /* 0x0C */      KEY_F4,
                /* 0x0D */      KEY_TAB,
                /* 0x0E */      KEY_BACK,
                /* 0x0F */      KEY_GRAVE,
                /* 0x10 */      KEY_RESERVED,
                /* 0x11 */      KEY_LEFTALT,
                /* 0x12 */      KEY_LEFTSHIFT,
                /* 0x13 */      KEY_RESERVED,
                /* 0x14 */      KEY_LEFTCTRL,
                /* 0x15 */      KEY_Q,
                /* 0x16 */      KEY_1,
                /* 0x17 */      KEY_RESERVED,
                /* 0x18 */      KEY_RESERVED,
                /* 0x19 */      KEY_RESERVED,
                /* 0x1A */      KEY_Z,
                /* 0x1B */      KEY_S,
                /* 0x1C */      KEY_A,
                /* 0x1D */      KEY_W,
                /* 0x1E */      KEY_2,
                /* 0x1F */      KEY_RESERVED,
                /* 0x20 */      KEY_RESERVED,
                /* 0x21 */      KEY_C,
                /* 0x22 */      KEY_X,
                /* 0x23 */      KEY_D,
                /* 0x24 */      KEY_E,
                /* 0x25 */      KEY_4,
                /* 0x26 */      KEY_3,
                /* 0x27 */      KEY_RESERVED,
                /* 0x28 */      KEY_RESERVED,
                /* 0x29 */      KEY_SPACE,
                /* 0x2A */      KEY_V,
                /* 0x2B */      KEY_F,
                /* 0x2C */      KEY_T,
                /* 0x2D */      KEY_R,
                /* 0x2E */      KEY_5,
                /* 0x2F */      KEY_RESERVED,
                /* 0x30 */      KEY_RESERVED,
                /* 0x31 */      KEY_N,
                /* 0x32 */      KEY_B,
                /* 0x33 */      KEY_H,
                /* 0x34 */      KEY_G,
                /* 0x35 */      KEY_Y,
                /* 0x36 */      KEY_6,
                /* 0x37 */      KEY_RESERVED,
                /* 0x38 */      KEY_RESERVED,
                /* 0x39 */      KEY_RESERVED,
                /* 0x3A */      KEY_M,
                /* 0x3B */      KEY_J,
                /* 0x3C */      KEY_U,
                /* 0x3D */      KEY_7,
                /* 0x3E */      KEY_8,
                /* 0x3F */      KEY_RESERVED,
                /* 0x40 */      KEY_RESERVED,
                /* 0x41 */      KEY_COMMA,
                /* 0x42 */      KEY_K,
                /* 0x43 */      KEY_I,
                /* 0x44 */      KEY_RESERVED,
                /* 0x45 */      KEY_0,
                /* 0x46 */      KEY_9,
                /* 0x47 */      KEY_RESERVED,
                /* 0x48 */      KEY_RESERVED,
                /* 0x49 */      KEY_DOT,
                /* 0x4A */      KEY_SLASH,
                /* 0x4B */      KEY_L,
                /* 0x4C */      KEY_SEMICOLON,
                /* 0x4D */      KEY_P,
                /* 0x4E */      KEY_MINUS,
                /* 0x4F */      KEY_RESERVED,
                /* 0x50 */      KEY_RESERVED,
                /* 0x51 */      KEY_RESERVED,
                /* 0x52 */      KEY_APOSTROPHE,
                /* 0x53 */      KEY_RESERVED,
                /* 0x54 */      KEY_LEFTBRACE,
                /* 0x55 */      KEY_EQUAL,
                /* 0x56 */      KEY_RESERVED,
                /* 0x57 */      KEY_RESERVED,
                /* 0x58 */      KEY_CAPSLOCK,
                /* 0x59 */      KEY_RIGHTSHIFT,
                /* 0x5A */      KEY_ENTER,
                /* 0x5B */      KEY_RESERVED,
                /* 0x5C */      KEY_RESERVED,
                /* 0x5D */      KEY_BACKSLASH,
                /* 0x5E */      KEY_RESERVED,
                /* 0x5F */      KEY_RESERVED,
                /* 0x60 */      KEY_RESERVED,
                /* 0x61 */      KEY_RESERVED,
                /* 0x62 */      KEY_RESERVED,
                /* 0x63 */      KEY_RESERVED,
                /* 0x64 */      KEY_RESERVED,
                /* 0x65 */      KEY_RESERVED,
                /* 0x66 */      KEY_BACKSPACE,
                /* 0x67 */      KEY_RESERVED,
                /* 0x68 */      KEY_RESERVED,
                /* 0x69 */      KEY_KP1,
                /* 0x6A */      KEY_RESERVED,
                /* 0x6B */      KEY_RIGHTBRACE,
                /* 0x6C */      KEY_KP7,
                /* 0x6D */      KEY_RESERVED,
                /* 0x6E */      KEY_RESERVED,
                /* 0x6F */      KEY_RESERVED,
                /* 0x70 */      KEY_KP0,
                /* 0x71 */      KEY_KPDOT,
                /* 0x72 */      KEY_KP2,
                /* 0x73 */      KEY_KP5,
                /* 0x74 */      KEY_KP6,
                /* 0x75 */      KEY_KP8,
                /* 0x76 */      KEY_ESC,
                /* 0x77 */      KEY_NUMLOCK,
                /* 0x78 */      KEY_F11,
                /* 0x79 */      KEY_KPPLUS,
                /* 0x7A */      KEY_KP3,
                /* 0x7B */      KEY_KPMINUS,
                /* 0x7C */      KEY_KPASTERISK,
                /* 0x7D */      KEY_KP9,
                /* 0x7E */      KEY_SCROLLLOCK,
                /* 0x7F */      KEY_RESERVED,
                /* 0x80 */      KEY_RESERVED,
                /* 0x81 */      KEY_RESERVED,
                /* 0x82 */      KEY_RESERVED,
                /* 0x83 */      KEY_F7
            ],
            ext_code_state: false,
            break_state: false,
            keyboard: None,
        }
    }
}


unsafe fn keyboard_scan_code_set2_handler(vector : u32)
{
    assert!(vector == 0x21);
    let scan_code = inb(KEYBOARD_DATA_PORT);
    let event;
    send_eoi(vector);
    if scan_code == 0xf0
    {
        SCAN_CODE_SET2.break_state = true;
        return;
    }
    if scan_code == 0xe0
    {
        SCAN_CODE_SET2.ext_code_state = true;
        return;
    }
    if SCAN_CODE_SET2.ext_code_state
    {
        todo!();
    }
    else {
        let value;
        if SCAN_CODE_SET2.break_state
        {
            value = 0;
            KEY_STATUS[SCAN_CODE_SET2.key_map[scan_code as usize] as usize] = false;
            SCAN_CODE_SET2.break_state = false;
        }
        else
        {
            if KEY_STATUS[SCAN_CODE_SET2.key_map[scan_code as usize] as usize]
            {
                value = 2;
            }
            else {
                KEY_STATUS[SCAN_CODE_SET2.key_map[scan_code as usize] as usize] = true;
                value = 1;
            }
        }
        event = InputEvent::new(EV_KEY, SCAN_CODE_SET2.key_map[scan_code as usize], value);
        logk!("{:?}\n", event);
    }
}