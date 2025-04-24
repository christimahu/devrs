# Troubleshooting DevRS

This guide covers common issues you might encounter when using DevRS and suggests solutions for each problem.

## Installation Issues

### `devrs` Command Not Found

**Problem:** After running `cargo install --path ./cli`, the `devrs` command isn't found.

**Solutions:**
- Ensure Cargo's bin directory is in your PATH: `export PATH="$HOME/.cargo/bin:$PATH"`
- Add this line to your shell configuration file (`.bashrc` or `.zshrc`)
- Verify by running `echo $PATH | grep cargo`
- Try restarting your terminal or running `source ~/.bashrc` (or `~/.zshrc`)

### Permission Error During `devrs setup`

**Problem:** The setup command fails with permission errors.

**Solutions:**
- Check file permissions for `~/.config` and `~/.local/share`
- Run with verbose output to see detailed errors: `devrs -vv setup`
- Try running individual setup steps to isolate the issue:
  ```bash
  devrs setup dependencies
  devrs setup shell
  devrs setup nvim
  ```

### Neovim Plugin Installation Fails

**Problem:** Neovim plugins don't install during setup.

**Solutions:**
- Ensure Neovim is installed on your system *before* running setup
- Check internet connectivity (required for plugin downloads)
- Manually run Neovim and try `:PackerSync`
- Remove Packer directory and try again:
  ```bash
  rm -rf ~/.local/share/nvim/site/pack/packer/start/packer.nvim
  devrs setup nvim --force
  ```

## Docker-Related Issues

### Docker Connection Errors

**Problem:** DevRS commands fail with Docker connectivity errors.

**Solutions:**
- Ensure Docker daemon is running:
  ```bash
  # Linux
  systemctl status docker
  # macOS/Windows
  open -a Docker
  ```
- Check Docker permission issues (Linux): Add your user to the `docker` group
- Restart Docker service if needed

### `devrs env build` Fails

**Problem:** The core environment image build fails.

**Solutions:**
- Check errors in the build output for specific failures
- Run with increased verbosity: `devrs -vv env build`
- Try building without cache: `devrs env build --no-cache`
- Ensure you're running from the DevRS repository root
- Check available disk space and free up space if needed
- Verify Docker has enough resource allocation (memory/CPU)

### Port Conflicts

**Problem:** Container commands fail with "port already allocated" errors.

**Solutions:**
- Find which process is using the conflicting port:
  ```bash
  # Linux/macOS
  lsof -i :8080
  # Windows
  netstat -ano | findstr 8080
  ```
- Stop the conflicting service or change the port mapping in config
- Edit your `~/.config/devrs/config.toml` to use different ports

### Missing Host Directories in Container

**Problem:** Directories mounted from host don't appear in container.

**Solutions:**
- Check mount paths in `~/.config/devrs/config.toml`
- Ensure your paths exist and have correct permissions
- Docker Desktop users: Verify file sharing is enabled for the mounted paths
- Check `devrs env status` for actual mounted volumes

## Blueprint Issues

### Blueprint Not Found

**Problem:** `devrs blueprint` commands can't find templates.

**Solutions:**
- Verify blueprint directory in config matches your DevRS repo location:
  ```bash
  devrs -vv blueprint list
  ```
- Check `~/.config/devrs/config.toml` and update the `blueprints.directory` path
- Ensure the repo contains the `blueprints` directory with templates
- Use an absolute path in config for blueprint directory

### Template Rendering Errors

**Problem:** `devrs blueprint create` fails with template errors.

**Solutions:**
- Check the template files for syntax errors
- Try with verbose logging: `devrs -vv blueprint create` 
- Verify template variable names match expected context
- Try a different template to narrow down if issue is template-specific

## Configuration Issues

### Configuration File Not Loading

**Problem:** Custom configuration doesn't seem to apply.

**Solutions:**
- Verify config file exists: `~/.config/devrs/config.toml`
- Check file permissions and ownership
- Validate TOML syntax with a linter
- Run with verbose logging: `devrs -vv env status`
- Look for parse errors in output

### Project-Specific Config Not Applied

**Problem:** `.devrs.toml` in project directory isn't overriding user config.

**Solutions:**
- Ensure `.devrs.toml` is in the correct directory
- Verify TOML syntax is valid
- Check if you need to fully specify all fields in a section to override it
- Run with verbose output to see config loading: `devrs -vv env status`

## General Troubleshooting Tips

- **Increase Verbosity:** Run commands with `-v`, `-vv`, or `-vvv` for more detailed logs
- **Check Docker Logs:** Use `docker logs <container_id>` for container-specific issues
- **Consult Help:** Use `devrs help <command>` for command-specific guidance
- **Check Config:** Verify your configuration with `cat ~/.config/devrs/config.toml`
- **Prune Docker Resources:** Occasionally run `docker system prune` to clean up unused resources
- **Verify PATH:** Ensure all required tools are in your PATH
- **Reset Config:** If all else fails, try removing `~/.config/devrs/config.toml` and running `devrs setup` again

If you encounter an issue not covered here, please search the [GitHub Issues](https://github.com/christimahu/devrs/issues) or open a new one.
