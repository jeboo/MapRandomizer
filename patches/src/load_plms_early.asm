; Modify the door transition code to change where PLMs get initialized and executed for 1 frame.
; Normally this happens in $82E4A9 near the end of the transition (often still in the middle of scrolling),
; which can result in artifacts (e.g. doors suddenly appearing, Chozo orbs suddenly disappearing).
; This would be more of an issue in the randomizer than in the vanilla game, because of the randomized
; doors, and especially because of how the orange color in Chozo orbs no longer fades to black.
; To avoid these problems, we set up PLMs at an earlier stage of the transition, in $82E36E.
; Since room setup and door ASM can spawn PLMs (particularly in the randomizer), we also move this
; (and some other room initialization) up to earlier in the transition as well.

!bank_80_free_space_start = $80E440
!bank_80_free_space_end = $80E500
!bank_82_free_space_start = $82FE00
!bank_82_free_space_end = $82FE40
!bank_84_free_space_start = $84EFD3
!bank_84_free_space_end = $84EFD7
!bank_8f_free_space_start = $8FFE50
!bank_8f_free_space_end = $8FFE80

; install a hook early in the door transition, just after level data and tile table are loaded and scrolling is set up:
org $80AD61 : JSR room_setup_hook  ; right door
org $80AD8B : JSR room_setup_hook  ; left door
org $80ADB5 : JSR room_setup_hook  ; down door
org $80ADE8 : JSR room_setup_hook  ; up door

; skip room setup later in the transition:
org $82E4B1
    PEA $8F00              ;\
    PLB                    ;} DB = $8F
    PLB
    JSL setup_asm_wrapper
    BRA skip_room_setup
warnpc $82E4C5
org $82E4C5
skip_room_setup:

org $82E4C9
    ; replaces JSR $E8EB  ; Spawn door closing PLM
    NOP : NOP : NOP

org $82E53C
    ; replaces JSL $8485B4  ; PLM handler
    JSL late_plm_handler

; hook code that runs regular setup ASM (not including extra setup ASM),
; to skip scrolling sky ASM (we delay it until after scrolling starts, since it takes so long).
org $8FE89D
    JSR setup_asm_hook

org !bank_82_free_space_start
spawn_closing_wrapper:
    JSR $E8EB    
    RTL
warnpc !bank_82_free_space_end

org !bank_80_free_space_start
room_setup_hook:
    JSR $AE29    ; run hi-jacked instruction: Update BG scroll offsets
    JSL $868016  ; Clear enemy projectiles
    JSL $878016  ; Clear animated tiles objects
    JSL $8DC4D8  ; Clear palette FX objects
    JSL $8483C3  ; Clear PLMs
    JSL $82EB6C  ; Create PLMs, execute door ASM, room setup ASM and set elevator status
    JSL spawn_closing_wrapper  ; Spawn door closing PLM
    
    ; early PLM handling: process PLMs before scrolling starts, with the exception of certain PLMs that
    ; must be processed later (e.g. because they depend on FX already being set up).
    ; this does something similar to function $8485B4, just skipping the exceptions.
early_plm_handler:
    PHB
    PEA $8484
    PLB
    PLB             ;} DB = $84
    BIT $1C23       ;\
    BPL .done       ;} If PLMs not enabled: return
    STZ $1C25       ; PLM draw tilemap index = 0
    LDX #$004E      ; X = 4Eh (PLM index)
.loop:
    STX $1C27       ; PLM index = [X]
    LDA $1C37,x     ;\
    BEQ .skip       ;} If [PLM ID] != 0:
    JSR is_delayed_plm
    BEQ .skip
    JSL process_plm_wrapper
    LDX $1C27       ; X = [PLM index]
.skip:
    DEX             ;\
    DEX             ;} X -= 2
    BPL .loop       ; If [X] >= 0: go to LOOP
    PLB
.done:
    RTS

    ; late PLM handling: process PLMs late, after the end of scrolling, like vanilla does.
    ; this only handles exceptional PLMs that were skipped during early handling.
late_plm_handler:
    PHB
    PEA $8484
    PLB
    PLB             ;} DB = $84
    BIT $1C23       ;\
    BPL .done       ;} If PLMs not enabled: return
    STZ $1C25       ; PLM draw tilemap index = 0
    LDX #$004E      ; X = 4Eh (PLM index)
.loop:
    STX $1C27       ; PLM index = [X]
    LDA $1C37,x     ;\
    BEQ .skip       ;} If [PLM ID] != 0:
    JSR is_delayed_plm
    BNE .skip
    JSL process_plm_wrapper
    LDX $1C27       ; X = [PLM index]
.skip:
    DEX             ;\
    DEX             ;} X -= 2
    BPL .loop       ; If [X] >= 0: go to LOOP
    PLB
.done:
    RTL

is_delayed_plm:
    CMP #$D70C      ; Glass Tunnel PLM
    BEQ .done
    CMP #$B777      ; Statues Room PLM to clear blocks
.done:
    RTS
warnpc !bank_80_free_space_end

org !bank_8f_free_space_start
; check if the setup ASM (in register A) is one whose execution should be delayed
; until the middle of scrolling (after loading the tilesets, etc.)
check_delayed_setup_asm:
    CMP #$91C9   ; scrolling sky
    BEQ .done
    CMP #$C11B   ; load glass tube tiles
.done:
    RTS

setup_asm_hook:
    JSR check_delayed_setup_asm
    BNE .run_setup
    LDA $0998
.check_state:
    CMP #$000B
    BEQ .skip      ; if in door transition state, skip running special setup ASM (such as scrolling sky)
.run_setup:
    JSR ($0018,x)  ; Execute (room setup ASM)
.skip:
    RTS

setup_asm_wrapper:
    LDX $07BB
    LDA $0018,x
    JSR check_delayed_setup_asm
    BNE .skip
    JSR ($0018,x)  ; Execute (room setup ASM)
.skip:
    RTL

warnpc !bank_8f_free_space_end

org !bank_84_free_space_start
process_plm_wrapper:
    JSR $85DA       ; Process PLM
    RTL
warnpc !bank_84_free_space_end