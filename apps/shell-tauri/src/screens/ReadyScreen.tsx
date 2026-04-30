import React from "react";

interface Props {
  appUrl: string;
}

export function ReadyScreen({ appUrl }: Props) {
  return (
    <div style={styles.container}>
      <iframe
        src={appUrl}
        style={styles.iframe}
        title="App Content"
      />
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    width: "100vw",
    height: "100vh",
    overflow: "hidden",
  },
  iframe: {
    width: "100%",
    height: "100%",
    border: "none",
  },
};
