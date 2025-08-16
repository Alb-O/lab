# Advanced Nix-based template engine
# This provides a more flexible templating system with variable interpolation
{ pkgs ? import <nixpkgs> {}
, lib ? pkgs.lib
}:

with lib;
with builtins;

rec {
  # Template variable syntax: {{variable-name}}
  templateVarPattern = "{{([a-zA-Z0-9_-]+)}}";
  
  # Extract all template variables from a string
  extractTemplateVars = content:
    let
      matches = match ".*{{([a-zA-Z0-9_-]+)}}.*" content;
    in
    if matches != null
    then [(head matches)] ++ (extractTemplateVars (replaceStrings ["{{${head matches}}}"] [""] content))
    else [];

  # Replace template variables in content
  interpolateTemplate = variables: content:
    foldl' (acc: varName:
      let
        placeholder = "{{${varName}}}";
        value = toString (variables.${varName} or placeholder);
      in
      replaceStrings [placeholder] [value] acc
    ) content (attrNames variables);

  # Create a template processor function
  createTemplateProcessor = { templatePath, defaultVariables ? {}, extraSubstitutions ? {} }:
    { variables ? {}, outputName ? "processed-template" }:
    let
      finalVariables = defaultVariables // variables;
      allSubstitutions = extraSubstitutions // (mapAttrs (n: v: toString v) finalVariables);
      
      templateFiles = lib.filesystem.listFilesRecursive templatePath;
      
      filteredFiles = filter (path: 
        let relPath = lib.removePrefix (toString templatePath + "/") (toString path);
        in !(lib.hasPrefix ".git" relPath || 
             lib.hasPrefix "result" relPath ||
             lib.hasPrefix "target" relPath ||
             lib.hasPrefix ".direnv" relPath ||
             lib.hasPrefix ".vscode" relPath)
      ) templateFiles;

      # Create sed script for template variable substitutions
      sedScript = concatStringsSep " " (mapAttrsToList (varName: value: 
        let
          # Use a delimiter that won't conflict with template syntax
          delimiter = "|";
          pattern = "{{${varName}}}";
          replacement = toString value;
        in
        "-e 's${delimiter}${pattern}${delimiter}${replacement}${delimiter}g'"
      ) finalVariables) + " " + 
      concatStringsSep " " (mapAttrsToList (from: to: 
        "-e 's|${escapeShellArg from}|${escapeShellArg to}|g'"
      ) extraSubstitutions);

    in
    pkgs.runCommand outputName {
      buildInputs = with pkgs; [ coreutils gnused file ];
    } ''
      mkdir -p "$out"
      
      echo "Processing template with variables:"
      ${concatStringsSep "\n" (mapAttrsToList (name: value: 
        "echo '  ${name}: ${toString value}'"
      ) finalVariables)}
      echo ""
      
      # Process each file
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
            # Process text files
            ${pkgs.gnused}/bin/sed ${sedScript} "${srcFile}" > "${destPath}"
            chmod --reference="${srcFile}" "${destPath}" 2>/dev/null || true
          fi
        ''
      ) filteredFiles}
      
      echo ""
      echo "Template processing complete!"
      echo "Output: $out"
    '';

  # Rust project template processor
  rustTemplate = createTemplateProcessor {
    templatePath = ../template-projects/rust-nix-template;
    defaultVariables = {
      package-name = "my-rust-project";
      author = "Your Name";
      author-email = "your.email@example.com";
      description = "A Rust project generated from template";
    };
    extraSubstitutions = {};
  };

  # Convenience function for quick project creation
  createRustProject = { name, author ? "Your Name", email ? "your.email@example.com", description ? null }:
    rustTemplate {
      variables = {
        package-name = name;
        author = author;
        author-email = email;
        description = if description != null then description else "A ${name} Rust project";
      };
      outputName = "rust-project-${name}";
    };
}