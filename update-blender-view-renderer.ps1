# PowerShell script to update BlenderViewRenderer.ts with new obsidian-logger calls
# This script replaces old debug calls with new pattern: debug(this, "human-readable message")

$filePath = "src\ui\components\BlenderViewRenderer.ts"

Write-Host "Updating BlenderViewRenderer.ts with new logger calls..." -ForegroundColor Green

# Check if file exists
if (-not (Test-Path $filePath)) {
	Write-Host "Error: File $filePath not found!" -ForegroundColor Red
	exit 1
}

# Read the file content
$content = Get-Content $filePath -Raw

# First, add the registerLoggerClass import and call
$content = $content -replace 'import \{ debug, info, warn, error \} from ''\.\.\/\.\.\/utils\/obsidian-logger'';', 'import { debug, info, warn, error, registerLoggerClass } from ''../../utils/obsidian-logger'';'

# Add registerLoggerClass call at the beginning of constructor (after the debug call)
$content = $content -replace "debug\('renderer', 'constructor:start'\);", "registerLoggerClass(this, 'BlenderViewRenderer');`n`t`tdebug(this, 'BlenderViewRenderer constructor started');"

# Replace all debug calls with new pattern and human-readable messages
$replacements = @{
	"debug\('renderer', 'constructor:settings-initialized', \{[^}]+\}\);" = "debug(this, 'Settings initialized from plugin configuration');"
	"debug\('renderer', 'constructor:creating-components'\);" = "debug(this, 'Creating UI component instances');"
	"debug\('renderer', 'constructor:setting-up-event-listeners'\);" = "debug(this, 'Setting up event listeners for build manager');"
	"info\('renderer', 'constructor:complete'\);" = "info(this, 'BlenderViewRenderer constructor completed successfully');"
	
	"debug\('renderer', 'initializeLayout:start', \{ isInitialized: this\.isInitialized \}\);" = "debug(this, ``Layout initialization started (already initialized: `${this.isInitialized})``);"
	"debug\('renderer', 'initializeLayout:already-initialized'\);" = "debug(this, 'Layout already initialized, skipping');"
	"debug\('renderer', 'initializeLayout:calling-initial-render'\);" = "debug(this, 'Calling initial render after layout initialization');"
	"info\('renderer', 'initializeLayout:complete'\);" = "info(this, 'Layout initialization completed successfully');"
	
	"debug\('renderer', 'render:start', \{ isInitialized: this\.isInitialized \}\);" = "debug(this, ``Main render started (initialized: `${this.isInitialized})``);"
	"debug\('renderer', 'render:not-initialized-calling-initializeLayout'\);" = "debug(this, 'Renderer not initialized, calling initializeLayout first');"
	"debug\('renderer', 'render:updating-toolbar'\);" = "debug(this, 'Updating toolbar section');"
	"debug\('renderer', 'render:updating-filter-section'\);" = "debug(this, 'Updating filter section');"
	"debug\('renderer', 'render:updating-status-display'\);" = "debug(this, 'Updating status display section');"
}

# Apply each replacement
foreach ($old in $replacements.Keys) {
	$new = $replacements[$old]
	$content = $content -replace $old, $new
	Write-Host "✓ Replaced: $old" -ForegroundColor Yellow
}

# Write the updated content back to the file
$content | Set-Content $filePath -NoNewline

Write-Host "✅ Successfully updated BlenderViewRenderer.ts!" -ForegroundColor Green
Write-Host "Updated the following:" -ForegroundColor Cyan
Write-Host "  - Added registerLoggerClass import" -ForegroundColor White
Write-Host "  - Added registerLoggerClass call in constructor" -ForegroundColor White
Write-Host "  - Updated $($replacements.Count) debug/info calls to use new pattern" -ForegroundColor White
Write-Host "  - All messages are now human-readable" -ForegroundColor White
