//! A proportional-integral-derivative (PID) controller implementation
//! 
//! This module provides a basic PID controller with optional anti-windup protection
//! and derivative filtering.

use std::time::{Instant, Duration};

/// A proportional-integral-derivative (PID) controller
#[derive(Debug, Clone, Copy)]
pub struct PidController {
    /// Proportional gain
    kp: f64,
    /// Integral gain
    ki: f64,
    /// Derivative gain
    kd: f64,
    /// Setpoint (target value)
    setpoint: f64,
    /// Integral term accumulation
    integral: f64,
    /// Previous error value
    prev_error: f64,
    /// Previous process variable
    prev_process_var: f64,
    /// Time of last update
    last_update: Instant,
    /// Output limits to prevent integral windup
    output_limits: Option<(f64, f64)>,
    /// Derivative filter coefficient (0.0 to 1.0)
    derivative_filter: f64,
    /// Whether to use derivative on measurement instead of error
    derivative_on_measurement: bool,
}

impl PidController {
    /// Creates a new PID controller with default parameters
    /// 
    /// # Arguments
    /// * `kp` - Proportional gain
    /// * `ki` - Integral gain
    /// * `kd` - Derivative gain
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            setpoint: 0.0,
            integral: 0.0,
            prev_error: 0.0,
            prev_process_var: 0.0,
            last_update: Instant::now(),
            output_limits: None,
            derivative_filter: 0.0,
            derivative_on_measurement: false,
        }
    }

    /// Sets the target setpoint for the controller
    pub fn set_setpoint(&mut self, setpoint: f64) {
        self.setpoint = setpoint;
    }

    /// Gets the current setpoint
    pub fn setpoint(&self) -> f64 {
        self.setpoint
    }

    /// Sets output limits to prevent windup and saturation
    pub fn set_output_limits(&mut self, limits: (f64, f64)) {
        self.output_limits = Some(limits);
    }

    /// Disables output limits
    pub fn disable_output_limits(&mut self) {
        self.output_limits = None;
    }

    /// Enables derivative filtering with the given coefficient
    /// 
    /// # Arguments
    /// * `alpha` - Filter coefficient (0.0 to 1.0), where 0.0 is no filtering
    pub fn set_derivative_filter(&mut self, alpha: f64) {
        self.derivative_filter = alpha.clamp(0.0, 1.0);
    }

    /// Enables or disables derivative-on-measurement mode
    /// 
    /// When true, derivative is calculated from the process variable instead of error
    pub fn set_derivative_on_measurement(&mut self, enabled: bool) {
        self.derivative_on_measurement = enabled;
    }

    /// Resets the controller state
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = 0.0;
        self.prev_process_var = 0.0;
        self.last_update = Instant::now();
    }

    /// Updates the PID controller with the current process variable and returns the control output
    /// 
    /// # Arguments
    /// * `process_var` - Current measured value of the process
    pub fn update(&mut self, process_var: f64) -> f64 {
        let now = Instant::now();
        let dt = (now - self.last_update).as_secs_f64();
        self.last_update = now;

        // Handle case where dt is zero or very small to prevent division issues
        if dt < 1e-6 {
            return self.calculate_output(0.0);
        }

        let error = self.setpoint - process_var;

        // Proportional term
        let p_term = self.kp * error;

        // Integral term with anti-windup protection
        let i_term = if let Some((min, max)) = self.output_limits {
            // Only integrate if we're not at the output limits
            let output = p_term + self.integral;
            if (output >= max && error > 0.0) || (output <= min && error < 0.0) {
                self.integral
            } else {
                self.integral + self.ki * error * dt
            }
        } else {
            self.integral + self.ki * error * dt
        };

        // Save integral for next iteration
        self.integral = i_term;

        // Derivative term
        let d_term = if self.kd > 0.0 && dt > 0.0 {
            if self.derivative_on_measurement {
                // Derivative on measurement
                let d_measurement = (process_var - self.prev_process_var) / dt;
                -self.kd * d_measurement
            } else {
                // Derivative on error
                let d_error = (error - self.prev_error) / dt;
                self.kd * d_error
            }
        } else {
            0.0
        };

        // Apply derivative filtering if enabled
        let d_term = if self.derivative_filter > 0.0 {
            self.prev_error * self.derivative_filter + d_term * (1.0 - self.derivative_filter)
        } else {
            d_term
        };

        // Save values for next iteration
        self.prev_error = error;
        self.prev_process_var = process_var;

        // Calculate and return the output with limits applied
        self.calculate_output(p_term + i_term + d_term)
    }

    /// Applies output limits if enabled
    fn calculate_output(&self, output: f64) -> f64 {
        if let Some((min, max)) = self.output_limits {
            output.clamp(min, max)
        } else {
            output
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_pid() {
        let mut pid = PidController::new(2.0, 0.5, 0.1);
        pid.set_setpoint(10.0);
        pid.set_output_limits((-100.0, 100.0));

        // Simulate a simple system where process_var = output * 0.1
        let mut process_var = 0.0;
        for _ in 0..100 {
            let output = pid.update(process_var);
            process_var += output * 0.1;
        }

        // After several iterations, process_var should be close to setpoint
        assert!((process_var - 10.0).abs() < 0.5);
    }

    #[test]
    fn test_integral_windup_protection() {
        let mut pid = PidController::new(1.0, 1.0, 0.0);
        pid.set_setpoint(100.0);
        pid.set_output_limits((-10.0, 10.0));

        // Process variable starts far from setpoint
        let process_var = 0.0;
        for _ in 0..10 {
            pid.update(process_var);
        }

        // Integral should not wind up beyond what's needed to reach output limit
        assert!((pid.integral - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_derivative_on_measurement() {
        let mut pid = PidController::new(1.0, 0.0, 1.0);
        pid.set_setpoint(10.0);
        pid.set_derivative_on_measurement(true);

        let output1 = pid.update(0.0);
        let output2 = pid.update(5.0);

        // With derivative on measurement, rapid increase in process_var should produce negative derivative term
        assert!(output2 < output1);
    }
}