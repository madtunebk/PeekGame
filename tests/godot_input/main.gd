extends Node

func _ready():
	var button = Button.new()
	button.position = Vector2(250, 140)
	button.size = Vector2(140, 80)
	button.text = "INPUT TARGET"
	button.pressed.connect(func(): print("INPUT_PROBE_PRESSED"))
	add_child(button)
	print("INPUT_PROBE_READY")

func _input(event):
	if event is InputEventMouseMotion or event is InputEventMouseButton:
		print("INPUT_PROBE_EVENT position=", event.position, " viewport=", get_viewport().get_visible_rect().size, " window=", DisplayServer.window_get_size(), " origin=", DisplayServer.window_get_position())
