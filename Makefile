SRC_DIR=src/*
TESTS_DIR=tests/*

format:
	isort $(SRC_DIR) $(TESTS_DIR)
	autoflake $(SRC_DIR) $(TESTS_DIR) --remove-all-unused-imports -r -i --ignore-init-module-imports
	black $(SRC_DIR) $(TESTS_DIR)

lint:
	pylint src/* --disable C0114,C0115,C0116,W0511,R0801,W0603,R0903

	autoflake src/* --remove-all-unused-imports -r --check --ignore-init-module-imports
	black src/* --check
	mypy src/sec_store

test: lint
	pytest tests --exitfirst