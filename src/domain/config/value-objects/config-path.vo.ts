/**
 * XDG-compliant config path resolution.
 * Follows the XDG Base Directory Specification.
 */

const APP_NAME = "smart-scribe"
const CONFIG_FILE = "config.toml"

/**
 * Get the config directory path.
 * Uses $XDG_CONFIG_HOME if set, otherwise falls back to ~/.config
 */
function getConfigDir(): string {
  const xdgHome = process.env.XDG_CONFIG_HOME
  const home = process.env.HOME ?? ""
  const base = xdgHome ?? `${home}/.config`
  return `${base}/${APP_NAME}`
}

/**
 * Get the full path to the config file.
 */
function getConfigFilePath(): string {
  return `${getConfigDir()}/${CONFIG_FILE}`
}

export const ConfigPath = {
  getConfigDir,
  getConfigFilePath,
}
