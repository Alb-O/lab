import logging
import bpy

# --- Configuration ---
ADDON_PACKAGE_NAME = ""  # This will be set by fly_nav.__init__.py or root __init__.py
LOG_LEVEL = logging.INFO  # Default log level, can be changed by preferences
# Prettier log format: [LEVEL | LOGGER_NAME] Message
LOG_FORMAT = '[%(levelname)s | %(name)s] %(message)s'
# Shorter date format, or remove if too verbose for console
DATE_FORMAT = '%H:%M:%S' # Example: 14:37:44

LOG_COLORS = {
    logging.DEBUG: '\033[95m',    # Magenta
    logging.INFO: '\033[94m',     # Blue
    logging.WARNING: '\033[93m',  # Yellow
    logging.ERROR: '\033[91m',    # Red
    logging.CRITICAL: '\033[91m\033[1m', # Bold Red
    'RESET': '\033[0m'
}

class ColoredFormatter(logging.Formatter):
    def __init__(self, fmt=None, datefmt=None, style='%', validate=True, *, defaults=None):
        # In Python 3.10+, validate defaults to True and style must be one of '%', '{', or '$'
        # For older versions, validate might not exist or style handling is different.
        # We explicitly pass style='%' which is the default for format strings like ours.
        super().__init__(fmt=fmt, datefmt=datefmt, style=style, defaults=defaults)
        # The `validate` argument was more problematic in 3.8/3.9 if not perfectly matched.
        # In 3.10+ `validate` is present. For broader compatibility, especially if Blender's Python is older,
        # being explicit with `style='%'` is safer.

    def format(self, record):
        log_message = super().format(record)
        color = LOG_COLORS.get(record.levelno)
        if color:
            return f"{color}{log_message}{LOG_COLORS['RESET']}"
        return log_message

# --- Logger Instance Cache ---
# Cache logger instances to avoid reconfiguring them
_loggers = {}

def get_logger(name="fly_nav"):
    """
    Retrieves a configured logger instance from cache or creates a new one.
    """
    global _loggers
    if name in _loggers:
        return _loggers[name]

    logger = logging.getLogger(name) # Use the provided name directly for the logger instance
    
    # Prevent duplicate handlers if the function is called multiple times (e.g., during reloads for the same name)
    if not logger.handlers:
        logger.setLevel(LOG_LEVEL)
        
        # Console Handler
        ch = logging.StreamHandler() # Outputs to stderr by default, which Blender shows in console
        ch.setLevel(LOG_LEVEL)
        
        # Formatter
        formatter = ColoredFormatter(LOG_FORMAT, datefmt=DATE_FORMAT) # New Colored Formatter
        ch.setFormatter(formatter)
        
        logger.addHandler(ch)
        
    _loggers[name] = logger
    return logger

def set_log_level(level):
    """
    Sets the logging level for all known loggers and the default for new ones.
    `level` can be a string like 'DEBUG', 'INFO', or a logging constant like logging.DEBUG.
    """
    global LOG_LEVEL
    if isinstance(level, str):
        new_level = getattr(logging, level.upper(), logging.INFO)
    else:
        new_level = level
    
    LOG_LEVEL = new_level
        
    for logger_instance in _loggers.values():
        logger_instance.setLevel(LOG_LEVEL)
        for handler in logger_instance.handlers:
            handler.setLevel(LOG_LEVEL)
    # Future loggers created via get_logger will also use this new LOG_LEVEL

# --- Convenience logging functions ---
def _get_effective_logger_name(module_name=None):
    """Helper to create a concise logger name."""
    # Use the last part of the package name for brevity, e.g., "fly_nav_ext"
    base_name = ADDON_PACKAGE_NAME.split('.')[-1] if ADDON_PACKAGE_NAME else "addon"
    if module_name:
        return f"{base_name}.{module_name}"
    return base_name

def log_info(message, module_name=None):
    logger_name = _get_effective_logger_name(module_name)
    actual_logger = get_logger(logger_name)
    actual_logger.info(message)

def log_warning(message, module_name=None):
    logger_name = _get_effective_logger_name(module_name)
    actual_logger = get_logger(logger_name)
    actual_logger.warning(message)

def log_error(message, module_name=None):
    logger_name = _get_effective_logger_name(module_name)
    actual_logger = get_logger(logger_name)
    actual_logger.error(message)

def log_debug(message, module_name=None):
    logger_name = _get_effective_logger_name(module_name)
    actual_logger = get_logger(logger_name)
    actual_logger.debug(message)

# Example of how to update ADDON_PACKAGE_NAME from outside
# This should be called from your addon's __init__.py
def set_package_name(name):
    global ADDON_PACKAGE_NAME
    if ADDON_PACKAGE_NAME != name: # Only update if changed, to avoid too much reconfiguration
        ADDON_PACKAGE_NAME = name
        # Note: Existing logger instances in _loggers will retain their old names.
        # This is generally fine as their configuration (level, handlers) can still be updated.
        # For a full rename, _loggers would need to be cleared or managed more intricately.

if __name__ == "__main__":
    # Example Usage (won't have ADDON_PACKAGE_NAME set here initially)
    set_package_name("fly_nav_ext.test_scope") # Simulate package name setting
    set_log_level("DEBUG")

    # Test default logger name (based on ADDON_PACKAGE_NAME's last part)
    log_info("Info from base logger.") 
    # Expected name: test_scope

    # Test with module_name
    log_debug("This is a debug message.", module_name="MainTest")
    # Expected name: test_scope.MainTest
    log_info("This is an info message.", module_name="MainTest")
    log_warning("This is a warning message.", module_name="AnotherModule")
    log_error("This is an error message.", module_name="AnotherModule")

    # Test direct get_logger usage
    custom_log = get_logger("MyCustomLogger")
    custom_log.info("Info from a custom logger instance.")
