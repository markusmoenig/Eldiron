import clsx from "clsx";
import Heading from "@theme/Heading";
import styles from "./styles.module.css";

import FeaturesContent from "./FeaturesContent.mdx";

export default function HomepageFeatures() {
  return (
    <section className={styles.features}>
      <div className={styles.featuresInner}>
        <FeaturesContent />
      </div>
    </section>
  );
}
