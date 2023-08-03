use ::text_io::read;
use chess::{Board, ChessMove};
use log::{info, LevelFilter};
use shallow_red_engine::{
    engine::enter_engine,
    managers::cache_manager::{Cache, CacheInputGrouping},
    utils::engine_interface::EngineSettings,
};
use std::{
    str::FromStr,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
    time::Duration,
};

use parking_lot::RwLock;
use timecontrol::thinking_time;
use tokio::task;

mod timecontrol;

#[tokio::main]
async fn main() {
    // Initialize values used throughout play
    let mut board: Board = Board::default(); // Initializes to newboard
    let mut moves_played: u8 = 0; // Moves played in game
    let mut stop_channel: Option<Sender<bool>> = None;

    // Set up the cache thread
    let cache_arc = Arc::new(RwLock::new(Cache::default()));
    let cache_arc_thread = cache_arc.clone();

    let (cache_tx, cache_rx) = Cache::generate_channel();

    let _tx_spare = cache_tx.clone(); // Keep a spare sender around to prevent the cache server from quitting

    let _cache_thread_hndl =
        thread::spawn(move || Cache::cache_manager_server(cache_arc_thread, cache_rx));

    let cache = CacheInputGrouping {
        cache_ref: cache_arc,
        cache_tx,
    };

    // Setup logging
    let _ = simple_logging::log_to_file("shallowred.log", LevelFilter::Info);
    info!("Shallow Red starting");

    loop {
        let uci_input: String = read!("{}\n");
        info!("Received << {}", uci_input);

        let uci_output: Option<String> = parse_input(
            uci_input,
            &mut board,
            &mut stop_channel,
            Some(cache.clone()),
            &mut moves_played,
        )
        .await;
        info!("Sent >> {:#?}", uci_output);

        // Only print out if we have a message
        if let Some(out) = uci_output {
            if out == *"quit".to_string() {
                break;
            } else {
                println!("{}", out)
            }
        };
    }
}

async fn parse_input(
    uci_input: String,
    board: &mut Board,
    stop_channel: &mut Option<Sender<bool>>,
    cache: Option<CacheInputGrouping>,
    moves_played: &mut u8,
) -> Option<String> {
    // Split input by whitespace
    let parsed_input: Vec<&str> = uci_input.split_whitespace().collect();

    match parsed_input[0] {
        "uci" => {
            *moves_played = 0;
            Some("info name shallow-red 0.1\nuciok".to_string())
        }
        "isready" => Some("readyok".to_string()),
        "ucinewgame" => {
            *board = Board::default();
            *moves_played = 0;
            None
        } // Wipe board
        "position" => {
            load_position(parsed_input, board);
            None
        }
        "go" => {
            // Get our current time
            let time_remaining = Duration::from_millis(match board.side_to_move() {
                chess::Color::White => parsed_input[2].parse::<u64>().unwrap(),
                chess::Color::Black => parsed_input[4].parse::<u64>().unwrap(),
            });

            // Create a channel for stopping the engine
            let (tx, rx): (Sender<bool>, Receiver<bool>) = mpsc::channel(); // Stop channel
            *stop_channel = Some(tx);

            let mut settings = EngineSettings::default();
            settings.stop_engine_rcv = Some(rx);
            settings.verbose = false;
            settings.cache_settings = cache;
            settings.time_limit = thinking_time(*moves_played, time_remaining);

            let board_run = board.clone(); // Copy the current board
            task::spawn(async move {
                // Spawn a long thread to monitor to run the engine, which returns the result when finished
                let engine_out = run_engine(board_run, settings).await;
                println!("{}", engine_out);
            });
            *moves_played += 1;
            None
        }
        "debuginternal" => {
            let debug_board: String = read!("{}\n");
            *board = Board::from_str(&debug_board).unwrap();
            None
        }
        "stop" => {
            match stop_channel {
                Some(stop_chan) => {
                    let _ = stop_chan.send(true);
                } // Send a stop to engine
                None => {} // Don't care
            };
            None
        }
        "quit" => Some("quit".to_string()),
        _ => None, // todo
    }
}

fn load_position(input: Vec<&str>, board: &mut Board) {
    for str_move in &input[1..] {
        match *str_move {
            "startpos" => *board = Board::default(),
            "moves" => {}
            _ => {
                let chessmove = ChessMove::from_str(str_move).expect("Move should be legal");
                *board = board.make_move_new(chessmove);
            }
        }
    }
}

async fn run_engine(board: Board, settings: EngineSettings) -> String {
    info!("Running search on board {}, with settings {:#?}", board.to_string(), settings);
    let (best_move, search_results) = enter_engine(board, settings).await;
    if let Some(results) = search_results { info!("Search finished with results: {:#?}", results) }
    "bestmove ".to_owned() + &best_move.to_string()
}

#[cfg(test)]
mod test {
    use super::*;
    use chess::Square;

    #[tokio::test]
    async fn test_uciok() {
        let input = "uci";
        let mut board = Board::default();
        let output = parse_input(input.to_string(), &mut board, &mut None, None, &mut 0)
            .await
            .unwrap();
        assert_eq!(output, "info name shallow-red 0.1\nuciok")
    }

    #[tokio::test]
    async fn test_readyok() {
        let input = "isready";
        let mut board = Board::default();
        let output = parse_input(input.to_string(), &mut board, &mut None, None, &mut 0)
            .await
            .unwrap();
        assert_eq!(output, "readyok")
    }

    #[tokio::test]
    async fn test_newgame() {
        let input = "ucinewgame";
        let mut board = Board::default();
        let output = parse_input(input.to_string(), &mut board, &mut None, None, &mut 0).await;
        assert_eq!(output, None)
    }

    #[tokio::test]
    async fn test_position() {
        let input = "position startpos moves e2e4";
        let mut board = Board::default();
        parse_input(input.to_string(), &mut board, &mut None, None, &mut 0).await;
        let board_e2e4 =
            Board::default().make_move_new(ChessMove::new(Square::E2, Square::E4, None));
        assert_eq!(board, board_e2e4);
    }

    #[tokio::test]
    async fn test_go() {
        let input_pos = "position startpos moves e2e4";
        let mut board = Board::default();
        parse_input(input_pos.to_string(), &mut board, &mut None, None, &mut 0).await;

        let input = "go wtime 600000 btime 600000";
        parse_input(input.to_string(), &mut board, &mut None, None, &mut 0).await;
    }

    #[tokio::test]
    async fn test_blunder() {
        let mut board =
            Board::from_str("r3r1k1/ppp3pp/4p3/1P6/4p3/b3P3/qBQ2PPP/3R1RK1 w - - 0 1").unwrap();
        let input = "go wtime 600000 btime 600000";
        parse_input(input.to_string(), &mut board, &mut None, None, &mut 0).await;
    }
}
