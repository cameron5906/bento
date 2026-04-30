export interface StatusResponse {
  appId: string;
  state: SupervisorState;
  message: string;
  progress: number;
  appUrl: string | null;
  error: UserFacingError | null;
  services: ServiceStatus[];
}

export type SupervisorState =
  | { status: "notInstalled" }
  | { status: "installedNotPrepared" }
  | { status: "checkingSystem" }
  | { status: "preparingRuntime" }
  | { status: "importingImages" }
  | { status: "creatingNetwork" }
  | { status: "creatingVolumes" }
  | { status: "startingServices" }
  | { status: "startingProxy" }
  | { status: "waitingForHealth" }
  | { status: "ready"; appUrl: string }
  | { status: "stopping" }
  | { status: "stopped" }
  | { status: "repairing" }
  | { status: "failedRecoverable"; error: UserFacingError }
  | { status: "failedBlocked"; error: UserFacingError }
  | { status: "uninstalling" };

export interface UserFacingError {
  code: string;
  severity: "recoverable" | "blocked";
  userTitle: string;
  userMessage: string;
  technicalMessage: string;
  actions: string[];
}

export interface ServiceStatus {
  name: string;
  state: "pending" | "starting" | "running" | "stopped" | "failed";
}

export type ShellScreen = "loading" | "ready" | "error" | "blocked";

export function getScreen(state: SupervisorState): ShellScreen {
  switch (state.status) {
    case "ready":
      return "ready";
    case "failedRecoverable":
      return "error";
    case "failedBlocked":
      return "blocked";
    default:
      return "loading";
  }
}
