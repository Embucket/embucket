.PHONY: build deploy test logs clean

# Function name (override with: make deploy FUNCTION_NAME=your-function)
FUNCTION_NAME ?= embucket-lambda

ENV_FILE ?= config/.env
IAM_ROLE ?= arn:aws:iam::767397688925:role/EmbucketLambdaExecRole
# --layer-arn required for flexibiity
# otel-collector-config goes second as it overrides original config  from opentelemetry-collector 
OTEL_COLLECTOR_LAYERS ?= \
	--layer-arn arn:aws:lambda:us-east-2:184161586896:layer:opentelemetry-collector-arm64-0_19_0:1\
	--layer-arn arn:aws:lambda:us-east-2:767397688925:layer:otel-collector-config:22\
	--layer-arn arn:aws:lambda:us-east-2:580247275435:layer:LambdaInsightsExtension-Arm64:33

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
	cd ../.. && cargo lambda deploy $(OTEL_COLLECTOR_LAYERS) --iam-role $(IAM_ROLE) --env-file $(ENV_FILE) --binary-name bootstrap $(FUNCTION_NAME)
	aws logs create-log-group --log-group-name "/aws/lambda/$(FUNCTION_NAME)" >/dev/null 2>&1 || true
	@$(MAKE) url FUNCTION_NAME=$(FUNCTION_NAME)

url:
	@test -n "$(FUNCTION_NAME)" || (echo "ERROR: Missing function name. Use: make url <function-name>" >&2; exit 1)
	@set -e; \
	echo "Ensuring Function URL config exists for $(FUNCTION_NAME) (auth: NONE)..." ; \
	aws lambda create-function-url-config --function-name "$(FUNCTION_NAME)" --auth-type NONE >/dev/null 2>&1 || \
	  aws lambda update-function-url-config --function-name "$(FUNCTION_NAME)" --auth-type NONE >/dev/null ; \
	echo "Ensuring public invoke permission exists..." ; \
	aws lambda add-permission --function-name "$(FUNCTION_NAME)" \
	  --statement-id AllowPublicURLInvoke \
	  --action lambda:InvokeFunctionUrl \
	  --principal "*" \
	  --function-url-auth-type NONE >/dev/null 2>&1 || true ; \
	URL="$$(aws lambda get-function-url-config --function-name "$(FUNCTION_NAME)" --query 'FunctionUrl' --output text)"; \
	echo "$$URL"

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
