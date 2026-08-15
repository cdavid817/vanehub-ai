export class DesktopVerificationError extends Error {
  constructor(status, message, details = {}) {
    super(message);
    this.name = "DesktopVerificationError";
    this.status = status;
    this.details = details;
  }
}
