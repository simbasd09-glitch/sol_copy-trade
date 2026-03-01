$Detached = $false
if ($args -contains '-Detached') { $Detached = $true }

$ImageName = "solsnipe"
Write-Host "Building Docker image '$ImageName'..."

docker build -t $ImageName .
if ($LASTEXITCODE -ne 0) {
    Write-Error "Docker build failed with exit code $LASTEXITCODE"
    exit $LASTEXITCODE
}

if (-not $Detached) {
    Write-Host "Running container interactively (press Ctrl+C to stop)..."
    docker run --rm -it -p 8080:8080 --env-file ./.env -v "${PWD}\config.toml:/app/config.toml:ro" $ImageName
} else {
    Write-Host "Running container detached as 'solsnipe'..."
    docker run -d --name solsnipe -p 8080:8080 --env-file ./.env -v "${PWD}\config.toml:/app/config.toml:ro" $ImageName
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to start detached container (exit $LASTEXITCODE)"
        exit $LASTEXITCODE
    }
    Write-Host "Container started. View logs with: docker logs -f solsnipe"
}
