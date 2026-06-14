arch snes.cpu
lorom

!bank_a9_freespace_start = $a9fd00  ; this address is referenced in patch.rs fn apply_mother_brain_fight_patches
!bank_a9_freeespace_end = $a9fd40

;; Hook to make Samus stand up before escape (in "Short" Mother Brain fight mode)

org !bank_a9_freespace_start
    lda #$0017             ;\ Make Samus stand up
    jsl $90f084            ;/
    lda #$0000             ;\
    jsl $808fc1            ;/ Queue music stop
    lda #$b1d5
    rts
    
assert pc() <= !bank_a9_freeespace_end