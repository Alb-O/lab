{ pkgs ? import <nixpkgs> {}
, template ? "rust-nix-template"
, templatePath ? ../templates/rust/rust-nix-template
, params ? {}
}:

with pkgs;
with builtins;
with lib;

let
  # Default template parameters
  defaultParams = {
    package-name = "my-rust-project";
    author = "Your Name";
    author-email = "your.email@example.com";
  };

  # Merge provided params with defaults
  finalParams = defaultParams // params;

  # Template substitution mappings
  substitutions = {
    "rust-nix-template" = finalParams.package-name;
    "Sridhar Ratnakumar" = finalParams.author;
    "srid@srid.ca" = finalParams.author-email;
  };

  # Get all files from template directory recursively  
  templateFiles = lib.filesystem.listFilesRecursive templatePath;

  # Filter out files we don't want to copy
  filteredFiles = filter (path: 
    let relPath = lib.removePrefix (toString templatePath + "/") (toString path);
    in !(lib.hasPrefix ".git" relPath || 
         lib.hasPrefix "result" relPath ||
         lib.hasPrefix "target" relPath ||
         lib.hasPrefix ".direnv" relPath)
  ) templateFiles;

  # Create sed script for all substitutions
  sedScript = concatStringsSep " " (mapAttrsToList (from: to: 
    "-e 's|${escapeShellArg from}|${escapeShellArg to}|g'"
  ) substitutions);

  # Create the instantiated template
  instantiatedTemplate = runCommand "instantiated-${finalParams.package-name}" {
    buildInputs = [ coreutils gnused file ];
  } ''
    mkdir -p "$out"
    
    # Copy and process each file
    ${concatMapStringsSep "\n" (srcFile:
      let
        relPath = lib.removePrefix (toString templatePath + "/") (toString srcFile);
        destPath = "$out/" + relPath;
        destDir = dirOf destPath;
      in ''
        echo "Processing: ${relPath}"
        mkdir -p "${destDir}"
        
        # Check if file is binary
        if file --mime-type "${srcFile}" | grep -q "application/\|image/\|audio/\|video/"; then
          # Copy binary files as-is
          cp "${srcFile}" "${destPath}"
        else
          # Process text files for placeholder replacement
          ${gnused}/bin/sed ${sedScript} "${srcFile}" > "${destPath}"
          chmod --reference="${srcFile}" "${destPath}" 2>/dev/null || true
        fi
      ''
    ) filteredFiles}
    
    echo "Template instantiation complete!"
    echo "Package name: ${finalParams.package-name}"
    echo "Author: ${finalParams.author} <${finalParams.author-email}>"
  '';

in
instantiatedTemplate