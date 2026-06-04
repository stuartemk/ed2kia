# Lexical Purge Script - Sprint 86
$files = Get-ChildItem -Recurse -Include *.rs,*.md,*.toml | Where-Object { $_.FullName -notmatch '\\(target|node_modules)\\' }
$count = 0
foreach ($file in $files) {
    $content = Get-Content $file.FullName -Raw -ErrorAction SilentlyContinue
    if ($content) {
        $original = $content
        $content = $content -replace 'Stuartian','Topological'
        $content = $content -replace 'stuartian','topological'
        $content = $content -replace 'Panspermia','Cosmic_Transmission'
        $content = $content -replace 'panspermia','cosmic_transmission'
        $content = $content -replace 'Gödelian','Undecidable'
        $content = $content -replace 'godelian','undecidable'
        $content = $content -replace 'Love=Zero','Divergence_Minimization'
        $content = $content -replace 'love=zero','divergence_minimization'
        $content = $content -replace 'Apoptosis','Byzantine_Eviction'
        $content = $content -replace 'apoptosis','byzantine_eviction'
        if ($content -ne $original) {
            Set-Content $file.FullName $content -NoNewline -Encoding UTF8
            $count++
        }
    }
}
Write-Host "Purged $count files"
