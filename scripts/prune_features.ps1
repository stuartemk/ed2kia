$content = Get-Content Cargo.toml -Raw
$lines = $content -split "`r?`n"
$startIdx = 132  # line 133 (0-indexed)
$endIdx = 910    # line 911 (0-indexed)
$newLines = $lines[0..($startIdx-1)] + @("[features]", "default = []", 'p2p = ["dep:libp2p"]', 'wasm = ["dep:wasmtime"]', 'cuda = ["candle-core/cuda"]') + $lines[($endIdx+1)..($lines.Length-1)]
$newLines -join "`n" | Set-Content Cargo.toml -NoNewline
