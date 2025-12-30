.PHONY: build deploy test logs clean

# Following vars to be set externally:
#  FUNCTION_NAME - override name of the lambda function
#  ENV_FILE - override env file, relatively to project root
#  FEATURES - cargo features to enable, comma separated
#  LAYERS - additional layer ARNs to include, comma separated
#  AWS_LAMBDA_ROLE_ARN - IAM role ARN for the lambda function
#  WITH_OTEL_CONFIG - otel config path, file from <projectroot>/config/ dir

FUNCTION_NAME ?= embucket-lambda
ENV_FILE ?= config/.env.lambda
OTEL_COLLECTOR_LAYER=arn:aws:lambda:us-east-2:184161586896:layer:opentelemetry-collector-arm64-0_19_0:1
OTEL_CONFIG_ENV_PARAM=$(if $(WITH_OTEL_CONFIG),--env-var OPENTELEMETRY_COLLECTOR_CONFIG_URI=/var/task/$(WITH_OTEL_CONFIG))

build:
	cd ../.. && cargo lambda build --release -p embucket-lambda --arm64 \
		$(if $(FEATURES),--features $(FEATURES)) \
		$(if $(WITH_OTEL_CONFIG),--include $(WITH_OTEL_CONFIG)) \
		-o zip --manifest-path crates/embucket-lambda/Cargo.toml

# Deploy to AWS (must run from workspace root for include paths to work)
deploy: build deploy-only public-url
	@echo "Deployed $(FUNCTION_NAME) to AWS"

# Quick deploy without rebuild
deploy-only:
	cd ../.. && cargo lambda deploy --admerge \
		--env-file $(ENV_FILE) \
		$(OTEL_CONFIG_ENV_PARAM) \
		$(if $(WITH_OTEL_CONFIG),--layer-arn $(OTEL_COLLECTOR_LAYER)) \
		$(if $(AWS_LAMBDA_ROLE_ARN),--iam-role $(AWS_LAMBDA_ROLE_ARN)) \
		$(if $(LAYERS),--layer-arn $(LAYERS)) \
		--binary-name bootstrap $(FUNCTION_NAME)
	aws logs create-log-group --log-group-name "/aws/lambda/$(FUNCTION_NAME)" >/dev/null 2>&1 || true

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

###############################
# Non-standard targets below #
###############################

public-url:
	echo "Ensuring Function URL config exists for $(FUNCTION_NAME) (auth: NONE)..." ; \
	aws lambda create-function-url-config \
		--function-name "$(FUNCTION_NAME)" \
		--auth-type NONE >/dev/null 2>&1 || \
	  aws lambda update-function-url-config --function-name "$(FUNCTION_NAME)" --auth-type NONE >/dev/null ; \
	echo "Ensuring public invoke permission exists..." ; \
	aws lambda add-permission --function-name "$(FUNCTION_NAME)" \
	  --statement-id AllowPublicURLInvoke \
	  --action lambda:InvokeFunctionUrl \
	  --principal "*" \
	  --function-url-auth-type NONE >/dev/null 2>&1 || true ; \
	URL="$$(aws lambda get-function-url-config --function-name "$(FUNCTION_NAME)" --query 'FunctionUrl' --output text)"; \
	echo "$$URL"

streaming: INVOKE_MODE=RESPONSE_STREAM
streaming: invoke-mode

buffered: INVOKE_MODE=BUFFERED
buffered: invoke-mode

invoke-mode:
	echo "Enabling $(INVOKE_MODE) invoke mode for $(FUNCTION_NAME)..."
	@aws lambda update-function-url-config \
  		--function-name "$(FUNCTION_NAME)" \
		--invoke-mode $(INVOKE_MODE)
