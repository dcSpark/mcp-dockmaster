import globals from "globals";
import pluginJs from "@eslint/js";
import tseslint from "typescript-eslint";
import pluginReact from "eslint-plugin-react";
import pluginPrettier from "eslint-config-prettier";
import pluginReactHooks from "eslint-plugin-react-hooks";

const fixedBrowserGlobals = Object.fromEntries(
  Object.entries({ ...globals.browser, ...globals.node }).map(
    ([key, value]) => [key.trim(), value],
  ),
);

/** @type {import('eslint').Linter.Config[]} */
export default [
  { files: ["**/*.{js,mjs,cjs,ts,jsx,tsx}"] },
  {
    ignores: [
      "src-tauri/**",
      "dist/**",
      "build/**",
      "public/**",
      "src/examples/**",
      // remove lib once its fixed
      "src/lib/**",
    ],
  },
  { languageOptions: { globals: fixedBrowserGlobals } },
  pluginJs.configs.recommended,
  ...tseslint.configs.recommended,
  pluginReact.configs.flat.recommended,
  pluginReact.configs.flat["jsx-runtime"],
  pluginPrettier,
  {
    plugins: {
      react: pluginReact,
      "react-hooks": pluginReactHooks,
    },
    settings: {
      react: {
        version: "detect",
      },
    },
    rules: {
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-unused-vars": "warn",
    },
  },
];
