mod common;

use common::{EARLY, LATE, run};
use ld60_machine::Cpu as _;

#[test]
fn da_carries_to_the_next_digit() {
    let (cpu, _) = run("LDI A, 9\nST M0\nLDI A, 8\nCLC\nADC M0\nDA\nHLT", LATE, 100);
    assert_eq!((cpu.a(), cpu.carry()), (7, true));
}

#[test]
fn ds_keeps_the_borrow() {
    let (cpu, _) = run("LDI A, 5\nST M0\nLDI A, 3\nCLC\nSBC M0\nDS\nHLT", LATE, 100);
    assert_eq!((cpu.a(), cpu.carry()), (8, true));
}

#[test]
fn early_chips_lose_the_borrow_on_zero() {
    let source = "LDI A, 9\nST M0\nLDI A, 0\nSEC\nSBC M0\nDS\nHLT";
    let (late, _) = run(source, LATE, 100);
    let (early, _) = run(source, EARLY, 100);
    assert_eq!((late.a(), late.carry()), (0, true));
    assert_eq!((early.a(), early.carry()), (0, false));
}

#[test]
fn inc_does_not_touch_carry() {
    let (cpu, _) = run("SEC\nLDI A, 0xFF\nINC\nHLT", LATE, 100);
    assert_eq!((cpu.a(), cpu.zero(), cpu.carry()), (0, true, true));
}

#[test]
fn djnz_with_zero_loops_256_times() {
    let (_, steps) = run("LDI B, 0\nloop: DJNZ loop\nHLT", LATE, 1000);
    assert_eq!(steps, 1 + 256);
}

#[test]
fn indexed_address_wraps_inside_ram() {
    let (cpu, _) = run("LDI B, 0x0E\nLDI A, 7\nST [B+3]\nHLT", LATE, 100);
    assert_eq!(cpu.ram().read(1), 7);
}

#[test]
fn third_call_overwrites_the_oldest_return() {
    let source = "
        CALL f1
        WAIT
f1:     CALL f2
        RET
f2:     CALL f3
        RET
f3:     RET
    ";
    let (_, steps) = run(source, LATE, 1000);
    assert_eq!(steps, 1000, "the program should never reach WAIT");
}

#[test]
fn in_rev_reports_the_revision() {
    let (cpu, _) = run("IN REV\nHLT", LATE, 100);
    assert_eq!(cpu.a(), 0x20);
}

#[test]
fn reserved_opcodes_act_as_nop() {
    let (cpu, _) = run(".db 0x0B, 0xFF\nLDI A, 5\nHLT", LATE, 100);
    assert_eq!(cpu.a(), 5);
}
