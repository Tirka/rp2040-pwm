use core::sync::atomic::Ordering;

use defmt::info;
// use embassy_rp::gpio::{AnyPin, Input, Pull};
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio_programs::rotary_encoder::{Direction, PioEncoder};
use embassy_rp::pwm::SetDutyCycle;
// use embassy_time::Timer;

use crate::{DUTY_CYCLE, PWM};

const DUTY_STEP: u8 = 2; // percent

// #[embassy_executor::task]
// pub async fn pressus(input_pin: AnyPin) {
//     let mut p = Input::new(input_pin, Pull::Up);
//     p.set_schmitt(true);

//     loop {
//         p.wait_for_low().await;

//         info!("pressed");

//         p.wait_for_high().await;
//         Timer::after_millis(10).await;
//     }
// }

#[embassy_executor::task]
pub async fn handle_encoder(mut encoder: PioEncoder<'static, PIO0, 0>) {
    loop {
        let mut new_duty = DUTY_CYCLE.load(Ordering::Acquire);
        match encoder.read().await {
            Direction::Clockwise => {
                new_duty += DUTY_STEP;
                if new_duty > 100 {
                    new_duty = 100
                }
            }
            Direction::CounterClockwise => {
                new_duty = new_duty.saturating_sub(DUTY_STEP);
            }
        };

        info!("Setting PWM to {}%", new_duty);
        DUTY_CYCLE.store(new_duty, Ordering::Release);
        PWM.lock()
            .await
            .as_mut()
            .unwrap()
            .set_duty_cycle_percent(new_duty)
            .unwrap();
    }
}
