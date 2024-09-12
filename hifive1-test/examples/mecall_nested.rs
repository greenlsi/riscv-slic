#![no_std]
#![no_main]

extern crate panic_halt;
extern crate riscv_slic;

use hifive1::{
    hal::{
        e310x::{self, CLINT},
        prelude::*,
        DeviceResources,
    },
    pin, sprintln,
};

// generate SLIC code for this example
riscv_slic::codegen!(pac = e310x, swi = [Soft1, Soft2, Soft3]);
use slic::SoftwareInterrupt; // Re-export of automatically generated enum of interrupts in previous macro
static mut flag: bool = false;

/// HW handler for MachineTimer interrupts triggered by CLINT.
#[riscv_rt::core_interrupt(CoreInterrupt::MachineTimer)]
fn machine_timer() {
    let mtimecmp = CLINT::mtimecmp0();
    mtimecmp.modify(|val| *val += CLINT::freq() as u64);
    sprintln!(" Timer IN");
    let mepc = riscv_slic::riscv::register::mepc::read();
    sprintln!("MEPC: {}", mepc);

    unsafe {
        flag = true;

        //riscv_slic::set_threshold(255);

        riscv_slic::nested(|| {
            riscv_slic::pend(SoftwareInterrupt::Soft1);
            sprintln!(" T1");
            riscv_slic::pend(SoftwareInterrupt::Soft2);
            sprintln!(" T2");
            riscv_slic::pend(SoftwareInterrupt::Soft3);
            sprintln!(" T3");
        });
        //riscv_slic::set_threshold(0);
    }
    sprintln!("MEPC: {}", mepc);
    sprintln!(" Timer OUT");
}

/// Handler for SoftHigh task (high priority).
#[allow(non_snake_case)]
#[no_mangle]
fn Soft1() {
    sprintln!(" +start Soft1");
    sprintln!(" -stop Soft1");
}

/// Handler for SoftMedium task (medium priority). This task pends both SoftLow and SoftHigh.
#[allow(non_snake_case)]
#[no_mangle]
fn Soft2() {
    sprintln!(" +start Soft2");
    sprintln!(" -stop Soft2");
}

/// Handler for SoftLow task (low priority).
#[allow(non_snake_case)]
#[no_mangle]
fn Soft3() {
    sprintln!(" +start Soft3");
    sprintln!(" -stop Soft3");
}

#[riscv_rt::entry]
fn main() -> ! {
    let resources = DeviceResources::take().unwrap();
    let peripherals = resources.peripherals;

    let clocks = hifive1::configure_clocks(peripherals.PRCI, peripherals.AONCLK, 64.mhz().into());
    let gpio = resources.pins;

    // Configure UART for stdout
    hifive1::stdout::configure(
        peripherals.UART0,
        pin!(gpio, uart0_tx),
        pin!(gpio, uart0_rx),
        115_200.bps(),
        clocks,
    );

    sprintln!("Configuring CLINT...");
    // First, we make sure that all PLIC the interrupts are disabled and set the interrupts priorities
    CLINT::disable();
    let mtimer = CLINT::mtimer();
    mtimer.mtimecmp0.write(CLINT::freq() as u64);
    mtimer.mtime.write(0);

    sprintln!("Configuring SLIC...");
    // make sure that interrupts are off
    riscv_slic::disable();
    riscv_slic::clear_interrupts();
    // Set priorities
    unsafe {
        riscv_slic::set_priority(SoftwareInterrupt::Soft1, 1); // low priority
        riscv_slic::set_priority(SoftwareInterrupt::Soft2, 2); // medium priority
        riscv_slic::set_priority(SoftwareInterrupt::Soft3, 3); // high priority
    }

    sprintln!("Enabling interrupts...");
    unsafe {
        riscv_slic::set_interrupts();
        CLINT::mtimer_enable();
        riscv_slic::enable();
    }

    sprintln!("Done!");

    loop {
        sprintln!("Waiting for interrupts...");
        //riscv_slic::riscv::asm::wfi();
        while unsafe { !flag } {}
        unsafe {
            flag = false;
        }
        sprintln!("Interrupt received!");
    }
}
