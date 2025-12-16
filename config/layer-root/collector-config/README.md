To update the layer config:
1. `cd config/layer-root`
2. `zip -r otel-collector-config-layer.zip collector-config`
3. `cd ../..`
4. `aws lambda publish-layer-version
--layer-name otel-collector-config  
--zip-file fileb://config/layer-root/otel-collector-config-layer.zip  
--compatible-runtimes provided.al2 provided.al2023   
--compatible-architectures arm64`




