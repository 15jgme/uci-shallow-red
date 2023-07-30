use ::text_io::read;
use chess::{Board, ChessMove};
use log::{info, warn, LevelFilter};
use shallow_red_engine::engine::enter_engine;
use std::{
    str::FromStr,
    sync::mpsc::{self, Receiver, Sender},
};
use tokio::task;

#[tokio::main]
async fn main() {
    // Initialize values used throughout play
    let mut board: Board = Board::default(); // Initializes to newboard

    let mut stop_channel: Option<Sender<bool>> = None;

    // Setup logging
    let _ = simple_logging::log_to_file("shallowred.log", LevelFilter::Info);
    info!("Shallow Red starting");

    loop {
        let uci_input: String = read!("{}\n");
        info!("Received << {}", uci_input);

        let uci_output: Option<String> =
            parse_input(uci_input, &mut board, &mut stop_channel).await;
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
) -> Option<String> {
    // Split input by whitespace
    let parsed_input: Vec<&str> = uci_input.split_whitespace().collect();

    match parsed_input[0] {
        "uci" => Some("info name shallow-red 0.1\nuciok".to_string()),
        "isready" => Some("readyok".to_string()),
        "ucinewgame" => {
            *board = Board::default();
            None
        } // Wipe board
        "position" => {
            load_position(parsed_input, board);
            None
        }
        "go" => {
            //Create a channel for stopping the engine
            let (tx, rx): (Sender<bool>, Receiver<bool>) = mpsc::channel(); // Stop channel
            *stop_channel = Some(tx);

            let board_run = board.clone(); // Copy the current board
            task::spawn(async move {
                // Spawn a long thread to monitor to run the engine, which returns the result when finished
                let engine_out = run_engine(board_run, Some(rx)).await;
                println!("{}", engine_out);
            });
            None
        }
        "stop" => {
            match stop_channel {
                Some(stop_chan) => {let _ = stop_chan.send(true);}, // Send a stop to engine
                None => {}, // Don't care
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
                let chessmove = ChessMove::from_str(&str_move).expect("Move should be legal");
                *board = board.make_move_new(chessmove);
            }
        }
    }
}

async fn run_engine(board: Board, receiver: Option<Receiver<bool>>) -> String {
    let (best_move, _) = enter_engine(board, false, receiver).await;
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
        let output = parse_input(input.to_string(), &mut board, &mut None).await.unwrap();
        assert_eq!(output, "info name shallow-red 0.1\nuciok")
    }

    #[tokio::test]
    async fn test_readyok() {
        let input = "isready";
        let mut board = Board::default();
        let output = parse_input(input.to_string(), &mut board, &mut None).await.unwrap();
        assert_eq!(output, "readyok")
    }

    #[tokio::test]
    async fn test_newgame() {
        let input = "ucinewgame";
        let mut board = Board::default();
        let output = parse_input(input.to_string(), &mut board, &mut None).await;
        assert_eq!(output, None)
    }

    #[tokio::test]
    async fn test_position() {
        let input = "position startpos moves e2e4";
        let mut board = Board::default();
        parse_input(input.to_string(), &mut board, &mut None).await;
        let board_e2e4 =
            Board::default().make_move_new(ChessMove::new(Square::E2, Square::E4, None));
        assert_eq!(board, board_e2e4);
    }

    #[tokio::test]
    async fn test_go() {
        let input_pos = "position startpos moves e2e4";
        let mut board = Board::default();
        parse_input(input_pos.to_string(), &mut board, &mut None).await;

        let input = "go wtime 6000 btime 6000";
        parse_input(input.to_string(), &mut board, &mut None)
            .await;
    }
}
