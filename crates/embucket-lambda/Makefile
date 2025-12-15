.PHONY: build deploy test logs clean

# Function name (override with: make deploy FUNCTION_NAME=your-function)
FUNCTION_NAME ?= embucket-lambda
# supported features: "streaming"
FEATURES_PARAM := $(if $(FEATURES),--features $(FEATURES))

build:
	cd ../.. && cargo lambda build --release --arm64 --manifest-path crates/embucket-lambda/Cargo.toml $(FEATURES_PARAM)

# Deploy to AWS (must run from workspace root for include paths to work)
deploy: build
	cd ../.. && cargo lambda deploy --layer-arn arn:aws:lambda:us-east-2:184161586896:layer:opentelemetry-collector-arm64-0_19_0:1 arn:aws:lambda:us-east-2:184161586896:layer:opentelemetry-collector-arm64-0_19_0:1 arn:aws:lambda:us-east-2:767397688925:layer:otel-collector-config:19
 --binary-name bootstrap $(FUNCTION_NAME)

# Quick deploy without rebuild
deploy-only:
	cd ../.. && cargo lambda deploy --layer-arn arn:aws:lambda:us-east-2:184161586896:layer:opentelemetry-collector-arm64-0_19_0:1 arn:aws:lambda:us-east-2:767397688925:layer:otel-collector-config:19
 --binary-name bootstrap $(FUNCTION_NAME)

# Watch locally for development
watch:
	cargo lambda watch

# Test with a local event
test:
	cargo lambda invoke --data-file test-event.json

# Tail lambda logs
logs:
	aws logs tail /aws/lambda/$(FUNCTION_NAME) --since 5m --follow

# Verify deployment with snow CLI
verify:
	snow sql -c lambda -q "SELECT 1 as test_column"

# Clean build artifacts
clean:
	cargo clean
