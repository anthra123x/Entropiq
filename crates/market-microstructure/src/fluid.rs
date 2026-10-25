/// Hydrodynamic and quantum fluid metrics for market liquidity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluidMetrics {
    pub liquidity_density: f64,
    pub sound_speed: f64,
    pub flow_velocity: f64,
    pub mach_number: f64,
    pub reynolds_number: f64,
    pub is_cavitation_risk: bool,
}

pub struct LiquidityFluidModel;

impl LiquidityFluidModel {
    /// Elasticity bulk modulus K of the order book
    pub const BULK_MODULUS: f64 = 500_000.0;
    /// Kinematic viscosity of typical market order flow
    pub const KINEMATIC_VISCOSITY: f64 = 0.05;

    /// Computes fluid dynamics metrics from microstructural variables.
    /// - `total_depth`: Combined Bid and Ask volume
    /// - `spread`: Current bid-ask spread
    /// - `price_delta`: Micro-price shift over interval
    /// - `dt_sec`: Time interval in seconds
    pub fn compute_metrics(
        total_depth: f64,
        spread: f64,
        price_delta: f64,
        dt_sec: f64,
    ) -> FluidMetrics {
        let eff_spread = spread.max(0.0001);
        let eff_dt = dt_sec.max(0.0001);

        // Liquidity density rho = Depth / Spread (volume per price dollar/cent)
        let liquidity_density = (total_depth / eff_spread).max(0.01);

        // Speed of sound in liquidity: c_s = sqrt(K / rho)
        let sound_speed = (Self::BULK_MODULUS / liquidity_density).sqrt();

        // Flow velocity v = |delta_P| / dt
        let flow_velocity = price_delta.abs() / eff_dt;

        // Mach number M = v / c_s
        let mach_number = flow_velocity / sound_speed.max(1e-6);

        // Reynolds number Re = (v * spread) / nu
        let reynolds_number = (flow_velocity * eff_spread) / Self::KINEMATIC_VISCOSITY;

        // Cavitation risk occurs when Mach >= 1.0 (flow exceeds liquidity replenishment)
        let is_cavitation_risk = mach_number >= 1.0;

        FluidMetrics {
            liquidity_density,
            sound_speed,
            flow_velocity,
            mach_number,
            reynolds_number,
            is_cavitation_risk,
        }
    }
}
