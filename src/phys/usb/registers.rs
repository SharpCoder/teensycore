// Registers
pub const USBCMD: u32 = 0x402E_0140;
pub const USBSTS: u32 = 0x402E_0144;
pub const USBINTR: u32 = 0x402E_0148;
pub const DEVICEADDR: u32 = 0x402E_0154;
pub const ENDPTLISTADDR: u32 = 0x402E_0158;
pub const USBMODE: u32 = 0x402E_01A8;
pub const PORTSC1: u32 = 0x402E_0184;
pub const ENDPTSETUPSTAT: u32 = 0x402E_01AC;
pub const ENDPTPRIME: u32 = 0x402E_01B0;
pub const ENDPTFLUSH: u32 = 0x402E_01B4;
pub const ENDPTSTAT: u32 = 0x402E_01B8;
pub const ENDPTCOMPLETE: u32 = 0x402E_01BC;
pub const ENDPTCTRL0: u32 = 0x402E_01C0;

// Interrupts
pub const USBINT: u32 = 1;
pub const USBERRINT: u32 = 2;
pub const PCI: u32 = 1 << 2;
pub const FRI: u32 = 1 << 3;
pub const SEI: u32 = 1 << 4;
pub const URI: u32 = 1 << 6;
pub const SRI: u32 = 1 << 7;
pub const SLI: u32 = 1 << 8;
pub const HCH: u32 = 1 << 12;
pub const NAKE: u32 = 1 << 16;
pub const TI0: u32 = 1 << 24;
pub const TI1: u32 = 1 << 25;
