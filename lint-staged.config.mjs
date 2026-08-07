export default {
  "*.{ts,tsx}": "eslint --fix --max-warnings=0 --no-warn-ignored",
  "*.{js,mjs}": "eslint --fix --max-warnings=0 --no-warn-ignored",
  // --edition must match the edition in src-tauri/Cargo.toml.
  "*.rs": "rustfmt --edition 2021",
};
