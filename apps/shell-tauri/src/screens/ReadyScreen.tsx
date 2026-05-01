import { useEffect } from "react";

interface Props {
  appUrl: string;
}

export function ReadyScreen({ appUrl }: Props) {
  useEffect(() => {
    // Navigate the entire webview to the app URL instead of using an iframe.
    // This gives the app full control over the window, including drag-and-drop,
    // keyboard shortcuts, and native browser features that iframes block.
    window.location.href = appUrl;
  }, [appUrl]);

  return (
    <div style={styles.container}>
      <p style={styles.text}>Opening app...</p>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    height: "100vh",
    background: "#0a0a0f",
  },
  text: {
    color: "#888",
    fontSize: 14,
  },
};
