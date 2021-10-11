.macro MOV_L reg, value
    mov \reg, #((\value) & 0xFFFF)
    movk \reg, #((\value) >> 16), lsl #16
.endm

.macro ADR_REL reg, sym
    adrp \reg, \sym
    add \reg, \reg, #:lo12:\sym
.endm

.macro ADR_ABS reg, sym
    movz \reg, #:abs_g3:\sym
    movk \reg, #:abs_g2_nc:\sym
    movk \reg, #:abs_g1_nc:\sym
    movk \reg, #:abs_g0_nc:\sym
.endm
