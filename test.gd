extends Node2D

var score: int = 0
var player_name: String = "Player"

func _ready() -> void:
	# Basic math commands
	TinyConsole.register_command(multiply, "multiply", "multiply two numbers")
	TinyConsole.register_command(add, "add", "add two numbers")
	TinyConsole.register_command(divide, "divide", "divide a by b")

	# Game-like commands
	TinyConsole.register_command(set_score, "set_score", "set player score")
	TinyConsole.register_command(get_score, "get_score", "display current score")
	TinyConsole.register_command(set_player_name, "set_player_name", "set player name")
	TinyConsole.register_command(greet, "greet", "greet the player")
	TinyConsole.register_command(roll_dice, "roll_dice", "roll an N-sided die")
	TinyConsole.register_command(countdown, "countdown", "print countdown from N")
	TinyConsole.register_command(repeat, "repeat", "repeat a message N times")
	TinyConsole.register_command(status, "status", "show all game state")

	# Autocomplete source for set_score
	TinyConsole.add_argument_autocomplete_source("set_score", 0,
		Callable(self, "score_suggestions"))

func multiply(a: float, b: float) -> void:
	TinyConsole.info("%.2f * %.2f = %.2f" % [a, b, a * b])

func add(a: float, b: float) -> void:
	TinyConsole.info("%.2f + %.2f = %.2f" % [a, b, a + b])

func divide(a: float, b: float) -> void:
	if b == 0.0:
		TinyConsole.error("Division by zero!")
		return
	TinyConsole.info("%.2f / %.2f = %.2f" % [a, b, a / b])

func set_score(value: int) -> void:
	score = value
	TinyConsole.info("Score set to %d" % score)

func get_score() -> void:
	TinyConsole.info("Current score: %d" % score)

func set_player_name(new_name: String) -> void:
	player_name = new_name
	TinyConsole.info("Player name set to: " + player_name)

func greet() -> void:
	TinyConsole.info("Hello, %s! Your score is %d." % [player_name, score])

func roll_dice(sides: int) -> void:
	if sides < 1:
		TinyConsole.error("Dice must have at least 1 side")
		return
	var result = randi_range(1, sides)
	TinyConsole.info("Rolled d%d: %d" % [sides, result])

func countdown(n: int) -> void:
	for i in range(n, 0, -1):
		TinyConsole.info(str(i) + "...")
	TinyConsole.info("Go!")

func repeat(message: String) -> void:
	TinyConsole.info(message)

func status() -> void:
	TinyConsole.info("--- Game Status ---")
	TinyConsole.info("  Player: " + player_name)
	TinyConsole.info("  Score: %d" % score)
	TinyConsole.info("-------------------")

func score_suggestions() -> Array:
	return [0, 10, 50, 100, 500, 1000]
