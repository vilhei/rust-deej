use embedded_graphics::{
    mono_font::{ascii::{FONT_6X10, FONT_8X13}, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor, primitives::{PrimitiveStyle, PrimitiveStyleBuilder},
};

pub const TEXT_STYLE: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X10)
    .text_color(BinaryColor::On)
    .build();

pub const TEXT_STYLE_BOLD: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_8X13)
    .text_color(BinaryColor::On)
    .build();

pub const OUTER_RECT_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_color(BinaryColor::On)
    .stroke_width(1)
    .fill_color(BinaryColor::Off)
    .build();

pub const FILL_RECT_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .fill_color(BinaryColor::On)
    .build();
