use std::time::Duration;

pub(crate) fn thinking_time(moves_played: u8, time_remaining: Duration) -> Duration {
    let game_moves_expected: u8 = 45; // Expect ~40 moves per game

    let moves_left = std::cmp::max(game_moves_expected - moves_played, 10); // Always assume we have 10 moves left 

    // Take the expected time left OR 1 second, whichever is greater
    std::cmp::max(time_remaining/(moves_left as u32), Duration::from_secs(1))
}

#[cfg(test)]
mod tests{
    use super::thinking_time;
    use std::time::Duration;

    #[test]
    fn test_thinking_time(){
        assert_eq!(thinking_time(5, Duration::from_secs(0)), Duration::from_secs(1)); // Minimum 1s
        assert_eq!(thinking_time(30, Duration::from_secs(30)), Duration::from_secs(2)); // 2sec per move
    }
}