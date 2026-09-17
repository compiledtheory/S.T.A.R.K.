fn get_threshold(plant: &String) -> f32 {
    return match plant.as_str() {
        "FICUS" => 35.0,
        _ => -1.0,
    };
}

pub fn notify_moisture_levels(plant: &String, moisture: f32) {
    if moisture < get_threshold(plant) {
        println!("Moisture levels to low on plant {}", plant);
    }
}