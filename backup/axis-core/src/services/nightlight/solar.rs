use std::f64::consts::PI;

const DEG_TO_RAD: f64 = PI / 180.0;
const RAD_TO_DEG: f64 = 180.0 / PI;

pub struct SolarTimes {
    pub sunrise: String,
    pub sunset: String,
}

pub fn calculate_sunrise_sunset(lat: f64, lon: f64, year: i32, month: u32, day: u32) -> Option<SolarTimes> {
    let n = day_of_year(year, month, day)?;

    let lng_hour = utc_offset(n, lon);
    let declination = sun_declination(n);

    let sunrise_time = calc_sun_time(lng_hour, declination, lat, true)?;
    let sunset_time = calc_sun_time(lng_hour, declination, lat, false)?;

    Some(SolarTimes {
        sunrise: format_time(sunrise_time),
        sunset: format_time(sunset_time),
    })
}

fn day_of_year(year: i32, month: u32, day: u32) -> Option<f64> {
    let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if month < 1 || month > 12 || day < 1 || day > 31 {
        return None;
    }
    let is_leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let mut dim = days_in_month;
    if is_leap {
        dim[1] = 29;
    }
    if day > dim[(month - 1) as usize] as u32 {
        return None;
    }
    let mut n = day as f64;
    for i in 0..(month - 1) {
        n += dim[i as usize] as f64;
    }
    Some(n)
}

fn utc_offset(n: f64, lng: f64) -> f64 {
    let b = 2.0 * PI * (n - 81.0) / 365.0;
    let eq_time = 9.87 * b.sin() - 7.53 * b.cos() - 1.5 * b.sin();
    12.0 - (4.0 * lng + eq_time) / 60.0
}

fn sun_declination(n: f64) -> f64 {
    23.45 * (360.0 * (n + 284.0) / 365.0 * DEG_TO_RAD).sin()
}

fn calc_sun_time(lng_hour: f64, decl: f64, lat: f64, is_sunrise: bool) -> Option<f64> {
    let lat_rad = lat * DEG_TO_RAD;
    let decl_rad = decl * DEG_TO_RAD;
    let cos_ha = (90.833 * DEG_TO_RAD).cos() / (lat_rad.cos() * decl_rad.cos())
        - lat_rad.tan() * decl_rad.tan();

    if cos_ha.abs() > 1.0 {
        return None;
    }

    let ha = cos_ha.acos() * RAD_TO_DEG / 15.0;
    if is_sunrise {
        Some(lng_hour - ha)
    } else {
        Some(lng_hour + ha)
    }
}

fn format_time(hours: f64) -> String {
    let mut h = hours;
    if h < 0.0 {
        h += 24.0;
    }
    if h >= 24.0 {
        h -= 24.0;
    }
    let hours = h.floor() as i32;
    let minutes = ((h - h.floor()) * 60.0).round() as i32;
    let minutes = if minutes == 60 { 0 } else { minutes };
    let hours = if minutes == 60 { hours + 1 } else { hours };
    format!("{:02}:{:02}", hours, minutes)
}
