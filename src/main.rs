#![no_std]
#![no_main]

mod button_duty;

use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    peripherals::PIO0,
    pio::{InterruptHandler, Pio},
    pio_programs::rotary_encoder::{PioEncoder, PioEncoderProgram},
    pwm::{Config, Pwm, SetDutyCycle},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use portable_atomic::AtomicU8;
use {defmt_rtt as _, panic_probe as _};

type PwmType = Mutex<ThreadModeRawMutex, Option<Pwm<'static>>>;

const DUTY_START: u8 = 100; // percent

static PWM: PwmType = Mutex::new(None);
static DUTY_CYCLE: AtomicU8 = AtomicU8::new(DUTY_START);

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Note: Pin 18 = Blue wire (PWM Signal)
    let mut pwm = Pwm::new_output_a(p.PWM_SLICE1, p.PIN_18, {
        let mut c = Config::default();
        let pwm_freq = 25_000; // Hz, our desired frequency
        let clock_freq = embassy_rp::clocks::clk_sys_freq();
        c.top = (clock_freq / pwm_freq) as u16 - 1;
        c
    });
    pwm.set_duty_cycle_percent(DUTY_START).unwrap();

    {
        *PWM.lock().await = Some(pwm);
    }

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);
    let prg = PioEncoderProgram::new(&mut common);
    let encoder = PioEncoder::new(&mut common, sm0, p.PIN_4, p.PIN_5, &prg);

    // spawner.must_spawn(button_duty::pressus(p.PIN_27.into()));
    spawner.must_spawn(button_duty::handle_encoder(encoder));
}
