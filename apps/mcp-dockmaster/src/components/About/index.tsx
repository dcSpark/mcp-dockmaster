import "./styles.css";

const About = () => {
  return (
    <div className="h-full px-6 flex flex-col gap-8 py-10 max-w-4xl mx-auto w-full text-sm text-muted-foreground">
      <h1 className="font-semibold tracking-tight text-2xl  text-foreground">About MCP Dockmaster</h1>
      <section className="about-section">
        <h2 className="text-lg font-semibold text-foreground">Overview</h2>
        <p>
          MCP Dockmaster is a powerful application management platform designed to simplify the deployment, management,
          and monitoring of AI applications and services. Our platform provides a seamless experience for developers and
          users alike.
        </p>
      </section>
      <section className="about-section">
        <h2 className="text-lg font-semibold text-foreground">Mission</h2>
        <p>
          Our mission is to democratize access to advanced AI tools and applications by providing an intuitive interface
          for managing complex software ecosystems. We believe in making technology accessible to everyone, regardless
          of their technical expertise.
        </p>
      </section>
      <section className="about-section">
        <h2 className="text-lg font-semibold text-foreground">Features</h2>
        <ul>
          <li>Easy application installation and management</li>
          <li>Centralized control panel for all your AI applications</li>
          <li>Access to a curated store of AI applications</li>
          <li>Simplified configuration and deployment</li>
          <li>Real-time monitoring and logging</li>
        </ul>
      </section>
      <section className="about-section">
        <h2 className="text-lg font-semibold text-foreground">Contact</h2>
        <p>For more information about MCP Dockmaster, please visit our website or contact our support team.</p>
      </section>
    </div>
  );
};

export default About;
