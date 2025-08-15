// Barrel file for utils
export * from './utils';
export * from '../context-menu/utils';
export { 
	loggerDebug, loggerInfo, loggerWarn, loggerError, 
	initLogger, registerLoggerClass,
	setNamespaceOverride, setColors, setDefaultLogLevel,
	setFormatTemplate, setCallbackFormatTemplate, setMessageColor,
	getColors, getDefaultLogLevel, getCurrentNamespace,
	getFormatTemplate, getCallbackFormatTemplate, getMessageColor
} from './obsidian-logger';
