.PHONY: build deploy test logs clean

# Function name (override with: make deploy FUNCTION_NAME=your-function)
FUNCTION_NAME ?=

# Allow: make deploy my-function   (without writing FUNCTION_NAME=...)
EXTRA_GOALS := $(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS))
ifneq ($(strip $(EXTRA_GOALS)),)
  FUNCTION_NAME := $(word 1,$(EXTRA_GOALS))
endif

# Swallow the extra goal(s) so make doesn't treat them as real targets
$(eval $(EXTRA_GOALS):;@:)

ENV_FILE ?= embucket-lambda/.env
IAM_ROLE ?= arn:aws:iam::767397688925:role/EmbucketLambdaExecRole
# --layer-arn required for flexibiity
OTEL_COLLECTOR_LAYER ?= arn:aws:lambda:us-east-2:184161586896:layer:opentelemetry-collector-arm64-0_19_0:1,arn:aws:lambda:us-east-2:767397688925:layer:otel-collector-config:19
# supported features: "streaming"
FEATURES_PARAM := $(if $(FEATURES),--features $(FEATURES))

build:
	cd ../.. && cargo lambda build --release -p embucket-lambda --arm64 -o zip --include config/metastore.yaml --manifest-path crates/embucket-lambda/Cargo.toml $(FEATURES_PARAM)

# Deploy to AWS (must run from workspace root for include paths to work)
deploy: build deploy-only
	@echo "Deployed $(FUNCTION_NAME) to AWS"

# Quick deploy without rebuild
deploy-only:
	@test -n "$(FUNCTION_NAME)" || (echo "ERROR: Missing function name. Use: make deploy <function-name>" >&2; exit 1)
	cd ../.. && cargo lambda deploy --layer-arn $(OTEL_COLLECTOR_LAYER) --iam-role $(IAM_ROLE) --env-file $(ENV_FILE) --binary-name bootstrap $(FUNCTION_NAME)

# Watch locally for development
watch:
	cargo lambda watch

# Test with a local event
test:
	cargo lambda invoke --data-file test-event.json

# Tail lambda logs
logs:
	@test -n "$(FUNCTION_NAME)" || (echo "ERROR: Missing function name. Use: make deploy <function-name>" >&2; exit 1)
	aws logs tail /aws/lambda/$(FUNCTION_NAME) --since 5m --follow

# Verify deployment with snow CLI
verify:
	snow sql -c lambda -q "SELECT 1 as test_column"

# Clean build artifacts
clean:
	cargo clean
