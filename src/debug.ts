// Debug logging system using namespace-based debugging
// Users can enable debug messages by running: window.DEBUG?.enable('blender-build-manager') in the console
// Or enable Console > Verbose mode to see console.debug() messages

// Type-safe window interface extension
interface BlenderBuildManagerWindow {
	DEBUG?: {
		enable(namespace: string): string;
		disable(namespace: string): string;
		enabled(namespace: string): boolean;
	};
	_blenderBuildManagerDebugEnabled?: boolean;
}

// Type-safe window casting
type WindowWithBlenderBuildManager = Window & BlenderBuildManagerWindow;

// Simple debug namespace implementation
const DEBUG_NAMESPACE = 'blender-build-manager';

// Simple flag-based approach for more reliability
function isDebugEnabledSimple(): boolean {
	if (typeof window === 'undefined') return false;
	return !!(window as unknown as WindowWithBlenderBuildManager)._blenderBuildManagerDebugEnabled;
}

function setDebugEnabled(enabled: boolean): void {
	if (typeof window !== 'undefined') {
		(window as unknown as WindowWithBlenderBuildManager)._blenderBuildManagerDebugEnabled = enabled;
	}
}

// Initialize simple DEBUG controller - force recreation for reliability
function ensureDebugController() {
	if (typeof window === 'undefined') return;
	
	const typedWindow = window as unknown as WindowWithBlenderBuildManager;
	
	// Create or override the DEBUG controller to ensure it works
	if (!typedWindow.DEBUG) {
		typedWindow.DEBUG = {
			enable: () => '',
			disable: () => '',
			enabled: () => false
		};
	}
	
	// Store original methods if they exist
	const originalEnable = typedWindow.DEBUG!.enable;
	const originalDisable = typedWindow.DEBUG!.disable;
	const originalEnabled = typedWindow.DEBUG!.enabled;
	
	typedWindow.DEBUG!.enable = function(namespace: string): string {
		// Handle our namespace
		if (namespace === DEBUG_NAMESPACE || namespace === '*') {
			setDebugEnabled(true);
			const message = `Debug enabled for namespace: ${namespace}`;
			return message;
		}
		
		// Call original if it exists for other namespaces
		if (originalEnable && typeof originalEnable === 'function') {
			return originalEnable.call(this, namespace);
		}
		
		return `Debug enabled for namespace: ${namespace}`;
	};
	
	typedWindow.DEBUG!.disable = function(namespace: string): string {
		// Handle our namespace
		if (namespace === DEBUG_NAMESPACE || namespace === '*') {
			setDebugEnabled(false);
			const message = `Debug disabled for namespace: ${namespace}`;
			return message;
		}
		
		// Call original if it exists for other namespaces
		if (originalDisable && typeof originalDisable === 'function') {
			return originalDisable.call(this, namespace);
		}
		return `Debug disabled for namespace: ${namespace}`;
	};
	
	typedWindow.DEBUG!.enabled = function(namespace: string): boolean {
		// Handle our namespace
		if (namespace === DEBUG_NAMESPACE) {
			return isDebugEnabledSimple();
		}
		if (namespace === '*') {
			return isDebugEnabledSimple(); // For wildcard, return our status
		}
		
		// Call original if it exists for other namespaces
		if (originalEnabled && typeof originalEnabled === 'function') {
			return originalEnabled.call(this, namespace);
		}
		return false;
	};
}

// Check if debugging is enabled for our namespace
function isDebugEnabled(): boolean {
	return isDebugEnabledSimple();
}

// Type-safe, namespace-based debug logging functions
export function blenderBuildManagerDebug(namespace: string, operation: string, data?: any) {
	if (isDebugEnabled()) {
		const prefix = `${namespace}:${operation}`;
		if (data !== undefined) {
			console.debug(`%c${DEBUG_NAMESPACE}`, 'color: #0066cc; font-weight: bold;', prefix, data);
		} else {
			console.debug(`%c${DEBUG_NAMESPACE}`, 'color: #0066cc; font-weight: bold;', prefix);
		}
	}
}

export function blenderBuildManagerInfo(namespace: string, operation: string, data?: any) {
	if (isDebugEnabled()) {
		const prefix = `${namespace}:${operation}`;
		if (data !== undefined) {
			console.info(`%c${DEBUG_NAMESPACE}`, 'color: #0066cc; font-weight: bold;', prefix, data);
		} else {
			console.info(`%c${DEBUG_NAMESPACE}`, 'color: #0066cc; font-weight: bold;', prefix);
		}
	}
}

export function blenderBuildManagerWarn(namespace: string, operation: string, data?: any) {
	if (isDebugEnabled()) {
		const prefix = `${namespace}:${operation}`;
		if (data !== undefined) {
			console.warn(`%c${DEBUG_NAMESPACE}`, 'color: #ff8800; font-weight: bold;', prefix, data);
		} else {
			console.warn(`%c${DEBUG_NAMESPACE}`, 'color: #ff8800; font-weight: bold;', prefix);
		}
	}
}

export function blenderBuildManagerError(namespace: string, operation: string, data?: any) {
	if (isDebugEnabled()) {
		const prefix = `${namespace}:${operation}`;
		if (data !== undefined) {
			console.error(`%c${DEBUG_NAMESPACE}`, 'color: #cc0000; font-weight: bold;', prefix, data);
		} else {
			console.error(`%c${DEBUG_NAMESPACE}`, 'color: #cc0000; font-weight: bold;', prefix);
		}
	}
}

// Initialize the debug controller when this module loads
ensureDebugController();
