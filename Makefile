formula-check: ## validate Homebrew formula syntax
	ruby -c ./HomebrewFormula/op-loader.rb

help: ## show available targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

.PHONY: formula-check help
