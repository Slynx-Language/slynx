# Styles Table

This document describes the table, thus, hardcoded numerics, that represent specific styles to be applied on a ui primitive component. This is a 16bit numeric value that represents an specific style, and thus, contain
semantic information to tell the backend how to apply that specific value.
Below it will be written as `NAME=Numeric expects Type`, where the Numeric is a Decimal number, and `Type` is the type its expected by the property. You can see at the UI types that will be shown here on UI_TYPES.md.


BACKGROUND_COLOR=0 expects uint32. The color in rgba. 8 bits per channel

FOREGROUND_COLOR=1 expects uint32. The color in rgba. 8 bits per channel

PADDING=2 expects uint32, uint32, uint32, uint32. The order of the parameters is Up, Right, Down, Bottom. 

MARGIN=3 expects uint32, uint32, uint32, uint32. The order of the paramaters is Up, Right, Down, Bottom

SIZE=4 expects uint32, uint32. The order of the parameters is X, Y

FONT_SIZE=5 expects f32. The font size in pixels. Relative amounts, such as rem, and em, etc, is calculated on the frontend

FONT_WEIGHT=6 expects uint16. The weight of the font, going from 0 to 1000.

OPACITY=7 expects f32. Opacity between 0..1, 0 being fully transparent and 1 fully opaque

BORDER=8 expects uint32, uint16, f32, the order is color, radius, width. Radius and width are defined in pixels

SHADOW=9 expects uint32, uint32, uint32, uint32 the order is color, offsetx, offsety, radius. offsetx, offsety and radius are determined in pixels
