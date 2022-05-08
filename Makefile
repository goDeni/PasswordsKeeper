format:
	black src/*

lint: format
	pylint src/* --disable C0114

test: lint
	pytest tests