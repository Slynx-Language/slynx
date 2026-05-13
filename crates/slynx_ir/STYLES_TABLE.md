# Styles Table

This document describes the table, thus, hardcoded numerics, that represent specific styles to be applied on a ui primitive component. This is a 16bit numeric value that represents an specific style, and thus, contain
semantic information to tell the backend how to apply that specific value.
Below it will be written as `NAME=Numeric expects Type`, where the Numeric is a Decimal number, and `Type` is the type its expected by the property

BACKGROUND_COLOR=0 expects Color
FOREGROUND_COLOR=1 expects Color
PADDING=2 expects Vec4
MARGIN=3 expects Vec4
SIZE=4 expects Vec4
FONT_SIZE=5 expects px
FONT_WEIGHT=6 expects u16
OPACITY=7 expects f32
BORDER=8 expects Border
SHADOW=9 expects Shadow
