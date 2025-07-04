//! Database seeder for sample content
//! 
//! This module provides sample public domain content for testing and demonstration

use sqlx::SqlitePool;

use crate::CodexResult;
use super::models::Document;
use super::queries::DocumentQueries;

/// Sample content seeder
pub struct ContentSeeder;

impl ContentSeeder {
    /// Seed the database with sample public domain content
    pub async fn seed_sample_content(pool: &SqlitePool) -> CodexResult<()> {
        tracing::info!("Seeding database with sample content...");
        
        let documents = Self::get_sample_documents();
        
        for document in documents {
            // Check if document already exists
            if let Ok(Some(_)) = DocumentQueries::get_by_id(pool, &document.id).await {
                tracing::debug!("Document '{}' already exists, skipping", document.title);
                continue;
            }
            
            DocumentQueries::create(pool, &document).await?;
            tracing::debug!("Created document: '{}'", document.title);
        }
        
        tracing::info!("Sample content seeding completed");
        Ok(())
    }
    
    /// Get sample documents (~5000 words total)
    fn get_sample_documents() -> Vec<Document> {
        vec![
            Self::create_philosophy_document(),
            Self::create_science_document(),
            Self::create_history_document(),
            Self::create_literature_document(),
            Self::create_technology_document(),
        ]
    }
    
    /// Create philosophy sample document
    fn create_philosophy_document() -> Document {
        let mut doc = Document::new(
            "Stoicism: A Guide to Ancient Wisdom".to_string(),
            r#"
# Stoicism: A Guide to Ancient Wisdom

## Introduction

Stoicism is an ancient philosophical school that originated in Athens around 300 BCE. Founded by Zeno of Citium, Stoicism teaches that virtue is the sole true good and that external things like wealth, health, and reputation are "indifferent" - neither good nor bad in themselves.

## Core Principles

### 1. Focus on What You Can Control

The fundamental Stoic principle is the dichotomy of control. According to Epictetus, "Some things are within our power, while others are not." We can control our thoughts, judgments, desires, and actions, but we cannot control external events, other people's actions, or the past and future.

### 2. Virtue as the Highest Good

Stoics believe that virtue (wisdom, justice, courage, and temperance) is the only true good. A virtuous person is happy regardless of external circumstances because their well-being depends on their character, not on external conditions.

### 3. Emotional Resilience

Stoicism teaches that our emotions are largely the result of our judgments about events, not the events themselves. By changing our judgments and perceptions, we can achieve emotional equilibrium and inner peace.

## Key Figures

**Marcus Aurelius (121-180 CE)**: Roman Emperor and philosopher who wrote "Meditations," a series of personal reflections on Stoic philosophy.

**Epictetus (50-135 CE)**: A former slave who became one of the most influential Stoic teachers. His teachings were compiled in the "Discourses" and the "Enchiridion."

**Seneca (4 BCE - 65 CE)**: Roman statesman and philosopher who wrote extensively on practical ethics and how to live well.

## Practical Applications

Stoicism offers practical tools for modern life:

- **Morning Reflection**: Begin each day by considering what challenges you might face and how you can respond virtuously.
- **Evening Review**: End each day by reflecting on your actions and decisions, learning from mistakes without harsh self-judgment.
- **Negative Visualization**: Occasionally imagine losing things you value to increase gratitude and reduce attachment.
- **The View from Above**: Take a cosmic perspective on your problems to gain clarity and reduce anxiety.

## Modern Relevance

Contemporary psychology has validated many Stoic principles. Cognitive Behavioral Therapy (CBT) shares the Stoic insight that our thoughts influence our emotions and behaviors. The practice of mindfulness also aligns with Stoic awareness of the present moment.

Stoicism remains relevant because it addresses universal human challenges: dealing with adversity, managing emotions, finding meaning, and living ethically. Its emphasis on personal responsibility and rational thinking provides a framework for resilience in an uncertain world.
            "#.trim().to_string(),
            "text/markdown".to_string(),
        );
        
        doc.id = "stoicism-guide-001".to_string();
        doc.summary = Some("An introduction to Stoic philosophy, covering core principles, key figures, and practical applications for modern life.".to_string());
        doc.author = Some("Classical Philosophy Collective".to_string());
        doc.category = Some("Philosophy".to_string());
        doc.set_tags(vec!["philosophy".to_string(), "stoicism".to_string(), "ancient wisdom".to_string(), "ethics".to_string()]);
        doc.reading_time = Some(8);
        doc.difficulty_level = Some(2);
        doc.language = "en".to_string();
        
        doc
    }
    
    /// Create science sample document
    fn create_science_document() -> Document {
        let mut doc = Document::new(
            "Quantum Computing: Principles and Applications".to_string(),
            r#"
# Quantum Computing: Principles and Applications

## Introduction

Quantum computing represents a paradigm shift in computational technology, leveraging the principles of quantum mechanics to process information in fundamentally new ways. Unlike classical computers that use bits (0 or 1), quantum computers use quantum bits or "qubits" that can exist in superposition states.

## Quantum Mechanics Fundamentals

### Superposition

In quantum mechanics, particles can exist in multiple states simultaneously until measured. A qubit can be in a superposition of both 0 and 1 states, allowing quantum computers to process multiple possibilities in parallel.

### Entanglement

Quantum entanglement is a phenomenon where qubits become correlated in such a way that the quantum state of each qubit cannot be described independently. This property enables quantum computers to perform certain calculations exponentially faster than classical computers.

### Quantum Interference

Quantum algorithms manipulate probability amplitudes to increase the likelihood of measuring correct answers while decreasing the probability of incorrect ones through constructive and destructive interference.

## Quantum Algorithms

### Shor's Algorithm

Developed by Peter Shor in 1994, this algorithm can efficiently factor large integers, which forms the basis of many cryptographic systems. A sufficiently large quantum computer running Shor's algorithm could break RSA encryption.

### Grover's Algorithm

Lov Grover's algorithm provides a quadratic speedup for searching unsorted databases. While a classical computer requires O(n) operations to search an unsorted list of n items, Grover's algorithm requires only O(√n) operations.

### Quantum Simulation

Quantum computers excel at simulating quantum systems, which could revolutionize our understanding of molecular behavior, drug discovery, and materials science.

## Current Challenges

### Quantum Decoherence

Qubits are extremely fragile and lose their quantum properties quickly due to environmental interference. Maintaining quantum coherence long enough to perform useful calculations remains a significant challenge.

### Error Rates

Current quantum computers have high error rates compared to classical computers. Quantum error correction requires thousands of physical qubits to create one logical qubit with acceptable error rates.

### Scalability

Building quantum computers with enough qubits to solve practical problems while maintaining low error rates is an ongoing engineering challenge.

## Applications and Future Prospects

### Cryptography

Quantum computers could break current encryption methods but also enable quantum cryptography, providing theoretically unbreakable communication through quantum key distribution.

### Drug Discovery

Quantum simulation of molecular interactions could accelerate drug discovery by allowing researchers to model complex biological systems more accurately.

### Optimization

Many real-world problems involve optimization, from logistics to financial modeling. Quantum algorithms could provide significant advantages for certain classes of optimization problems.

### Artificial Intelligence

Quantum machine learning algorithms could potentially provide exponential speedups for certain AI tasks, though this remains an active area of research.

## Conclusion

Quantum computing is still in its early stages, but the potential applications are revolutionary. As the technology matures and overcomes current limitations, it promises to transform fields ranging from cryptography to drug discovery, ushering in a new era of computational capability.
            "#.trim().to_string(),
            "text/markdown".to_string(),
        );
        
        doc.id = "quantum-computing-001".to_string();
        doc.summary = Some("An overview of quantum computing principles, algorithms, challenges, and future applications.".to_string());
        doc.author = Some("Future Tech Research Group".to_string());
        doc.category = Some("Science & Technology".to_string());
        doc.set_tags(vec!["quantum computing".to_string(), "physics".to_string(), "technology".to_string(), "algorithms".to_string()]);
        doc.reading_time = Some(10);
        doc.difficulty_level = Some(4);
        doc.language = "en".to_string();
        
        doc
    }
    
    /// Create history sample document
    fn create_history_document() -> Document {
        let mut doc = Document::new(
            "The Scientific Revolution: Transforming Human Understanding".to_string(),
            r#"
# The Scientific Revolution: Transforming Human Understanding

## Overview

The Scientific Revolution, spanning roughly from 1543 to 1687, fundamentally transformed how humans understood the natural world. This period saw the emergence of modern scientific methodology and revolutionary discoveries that challenged medieval worldviews.

## Key Developments

### The Heliocentric Model

Nicolaus Copernicus (1473-1543) proposed that the Earth orbits the Sun, contradicting the geocentric model that had dominated for over a millennium. His work "De revolutionibus orbium coelestium" (1543) laid the foundation for modern astronomy.

### Galileo's Observations

Galileo Galilei (1564-1642) used the newly invented telescope to make groundbreaking observations:
- Discovered Jupiter's four largest moons
- Observed the phases of Venus
- Studied lunar craters and surface features
- Confirmed the heliocentric model through empirical evidence

### Kepler's Laws of Planetary Motion

Johannes Kepler (1571-1630) discovered that planetary orbits are elliptical rather than circular, providing mathematical precision to the heliocentric model. His three laws of planetary motion became fundamental principles of celestial mechanics.

### Newton's Synthesis

Isaac Newton (1643-1727) unified terrestrial and celestial mechanics with his law of universal gravitation. His "Principia Mathematica" (1687) established the mathematical foundation of classical physics and demonstrated that the same laws govern motion on Earth and in the heavens.

## Methodological Innovations

### Empirical Observation

The Scientific Revolution emphasized observation and experimentation over reliance on ancient authorities. Scientists began to trust their senses and instruments over inherited wisdom.

### Mathematical Description

Natural phenomena were increasingly described using mathematical relationships, allowing for precise predictions and quantitative analysis.

### Systematic Experimentation

Controlled experiments became the gold standard for testing hypotheses, establishing cause-and-effect relationships with greater reliability.

## Institutional Changes

### Royal Societies

Scientific academies like the Royal Society of London (1660) and the French Academy of Sciences (1666) provided forums for sharing research and establishing scientific standards.

### Scientific Communication

The development of scientific journals enabled rapid dissemination of discoveries and fostered international collaboration among researchers.

### Patronage Systems

Wealthy patrons and governments began supporting scientific research, recognizing its potential for technological and military advantages.

## Philosophical Implications

### Mechanical Universe

The Scientific Revolution promoted a mechanistic view of nature, where natural phenomena could be explained through mathematical laws and mechanical principles.

### Separation of Science and Religion

While many scientists remained religious, the Scientific Revolution began to separate natural philosophy from theology, establishing science as an independent domain of knowledge.

### Human Agency

The success of scientific methods gave humans confidence in their ability to understand and potentially control natural forces, fostering ideas about progress and human capability.

## Legacy and Impact

The Scientific Revolution established the foundations of modern science and technology. Its emphasis on empirical evidence, mathematical description, and systematic investigation continues to drive scientific progress today. The period also contributed to Enlightenment thinking about reason, progress, and human potential.

The transformation wasn't merely intellectual—it had profound social, political, and cultural consequences that continue to shape our world. The Scientific Revolution represents one of the most significant turning points in human history, marking the transition from medieval to modern ways of understanding nature and our place within it.
            "#.trim().to_string(),
            "text/markdown".to_string(),
        );
        
        doc.id = "scientific-revolution-001".to_string();
        doc.summary = Some("Exploration of the Scientific Revolution's key discoveries, methodological innovations, and lasting impact on human understanding.".to_string());
        doc.author = Some("Historical Research Institute".to_string());
        doc.category = Some("History".to_string());
        doc.set_tags(vec!["history".to_string(), "science".to_string(), "revolution".to_string(), "astronomy".to_string(), "methodology".to_string()]);
        doc.reading_time = Some(12);
        doc.difficulty_level = Some(3);
        doc.language = "en".to_string();
        
        doc
    }
    
    /// Create literature sample document
    fn create_literature_document() -> Document {
        let mut doc = Document::new(
            "The Hero's Journey: A Universal Narrative Pattern".to_string(),
            r#"
# The Hero's Journey: A Universal Narrative Pattern

## Introduction

Joseph Campbell's concept of the "Hero's Journey" or "Monomyth" describes a common narrative pattern found in myths, legends, and stories across cultures. This archetypal structure reveals fundamental aspects of human psychology and the universal themes that resonate across time and culture.

## The Three Acts

### Departure (The Call to Adventure)

The hero begins in the ordinary world but receives a call to adventure that disrupts their normal life. Initially, the hero may refuse the call, but eventually accepts the challenge, often with the help of a mentor figure.

**Key Stages:**
- The Ordinary World
- The Call to Adventure
- Refusal of the Call
- Meeting the Mentor
- Crossing the First Threshold

### Initiation (Trials and Transformation)

The hero enters a special world filled with challenges, allies, and enemies. Through tests and trials, the hero gains new powers, faces their greatest fear, and ultimately achieves their goal, though often at great cost.

**Key Stages:**
- Tests, Allies, and Enemies
- Approach to the Inmost Cave
- The Ordeal
- Reward (Seizing the Sword)

### Return (Integration and Wisdom)

The hero returns to the ordinary world, transformed by their experience. They must integrate their new wisdom and often share their knowledge or power with their community.

**Key Stages:**
- The Road Back
- Resurrection
- Return with the Elixir

## Psychological Dimensions

### Archetypal Characters

Campbell identified recurring character types that represent different aspects of the psyche:
- **The Hero**: The ego or conscious self undertaking growth
- **The Mentor**: Wisdom and guidance (often representing the higher self)
- **The Shadow**: The repressed or denied aspects of personality
- **The Threshold Guardian**: Tests and challenges that promote growth
- **The Shapeshifter**: Uncertainty and the anima/animus

### Universal Themes

The Hero's Journey addresses fundamental human experiences:
- Coming of age and personal growth
- Facing fears and overcoming obstacles
- The tension between safety and adventure
- Death and rebirth (psychological transformation)
- The relationship between individual and community

## Examples Across Cultures

### Ancient Myths
- **Greek**: Odysseus's journey home from Troy
- **Mesopotamian**: Gilgamesh's quest for immortality
- **Celtic**: The Welsh Mabinogion tales
- **Hindu**: Rama's exile and return in the Ramayana

### Modern Stories
- **Literature**: Harry Potter series, Lord of the Rings
- **Film**: Star Wars, The Matrix, Lion King
- **Traditional Tales**: Cinderella, Beauty and the Beast

## Applications and Significance

### Storytelling and Writing

Understanding the Hero's Journey helps writers create compelling narratives that resonate with audiences. The structure provides a framework for character development and plot progression.

### Psychology and Personal Development

The monomyth can be viewed as a metaphor for psychological growth and individuation. Many people recognize their own life experiences in the pattern of challenge, transformation, and integration.

### Cultural Understanding

The universality of the Hero's Journey suggests shared human experiences and values across cultures, providing insight into what connects us as a species.

### Educational Value

Teaching the Hero's Journey helps students understand narrative structure, recognize patterns in literature, and connect stories to broader themes about human experience.

## Criticisms and Limitations

Some scholars argue that Campbell's pattern:
- Oversimplifies diverse cultural narratives
- Reflects Western, male-centered perspectives
- Forces stories into artificial patterns
- Ignores cultural specificity and context

Despite these criticisms, the Hero's Journey remains a valuable tool for understanding narrative structure and the psychological dimensions of storytelling.

## Conclusion

The Hero's Journey reveals the power of narrative to express universal human experiences. Whether viewed as literary structure, psychological metaphor, or cultural artifact, Campbell's monomyth continues to illuminate the stories we tell and the journeys we undertake in our own lives.
            "#.trim().to_string(),
            "text/markdown".to_string(),
        );
        
        doc.id = "heros-journey-001".to_string();
        doc.summary = Some("Analysis of Joseph Campbell's Hero's Journey pattern in mythology, literature, and its psychological significance.".to_string());
        doc.author = Some("Narrative Studies Collective".to_string());
        doc.category = Some("Literature & Culture".to_string());
        doc.set_tags(vec!["literature".to_string(), "mythology".to_string(), "psychology".to_string(), "narrative".to_string(), "culture".to_string()]);
        doc.reading_time = Some(11);
        doc.difficulty_level = Some(2);
        doc.language = "en".to_string();
        
        doc
    }
    
    /// Create technology sample document
    fn create_technology_document() -> Document {
        let mut doc = Document::new(
            "Machine Learning Fundamentals: From Algorithms to Applications".to_string(),
            r#"
# Machine Learning Fundamentals: From Algorithms to Applications

## What is Machine Learning?

Machine Learning (ML) is a subset of artificial intelligence that enables computers to learn and improve from experience without being explicitly programmed. Instead of following pre-programmed instructions, ML systems identify patterns in data and make predictions or decisions based on those patterns.

## Types of Machine Learning

### Supervised Learning

In supervised learning, algorithms learn from labeled training data to make predictions on new, unseen data.

**Key Algorithms:**
- **Linear Regression**: Predicts continuous values by finding the best-fitting line through data points
- **Decision Trees**: Creates a tree-like model of decisions based on feature values
- **Random Forest**: Combines multiple decision trees for improved accuracy
- **Support Vector Machines (SVM)**: Finds optimal boundaries between different classes
- **Neural Networks**: Models inspired by biological neural networks

**Applications:**
- Image recognition and classification
- Email spam detection
- Medical diagnosis
- Credit scoring
- Weather forecasting

### Unsupervised Learning

Unsupervised learning finds hidden patterns in data without labeled examples.

**Key Algorithms:**
- **K-Means Clustering**: Groups similar data points together
- **Hierarchical Clustering**: Creates tree-like cluster structures
- **Principal Component Analysis (PCA)**: Reduces data dimensionality while preserving important information
- **Association Rules**: Discovers relationships between different variables

**Applications:**
- Customer segmentation
- Recommendation systems
- Anomaly detection
- Data compression
- Market basket analysis

### Reinforcement Learning

Reinforcement learning trains agents to make decisions through trial and error, receiving rewards or penalties for their actions.

**Key Concepts:**
- **Agent**: The learning system
- **Environment**: The context in which the agent operates
- **Actions**: Possible moves the agent can make
- **Rewards**: Feedback on action quality
- **Policy**: Strategy for choosing actions

**Applications:**
- Game playing (chess, Go, video games)
- Autonomous vehicles
- Robotics
- Trading algorithms
- Resource allocation

## The Machine Learning Process

### 1. Data Collection and Preparation

Quality data is crucial for ML success. This involves:
- Gathering relevant data from various sources
- Cleaning and preprocessing data
- Handling missing values and outliers
- Feature engineering and selection

### 2. Model Selection and Training

Choose appropriate algorithms based on:
- Problem type (classification, regression, clustering)
- Data characteristics (size, complexity, noise level)
- Performance requirements
- Interpretability needs

### 3. Evaluation and Validation

Assess model performance using:
- **Cross-validation**: Testing on multiple data subsets
- **Performance metrics**: Accuracy, precision, recall, F1-score
- **Confusion matrices**: Detailed breakdown of prediction results
- **ROC curves**: Trade-offs between true and false positive rates

### 4. Deployment and Monitoring

- Deploy models in production environments
- Monitor performance over time
- Retrain models as new data becomes available
- Handle model drift and changing patterns

## Key Challenges

### Overfitting and Underfitting

- **Overfitting**: Model learns training data too specifically, failing to generalize
- **Underfitting**: Model is too simple to capture underlying patterns
- **Solutions**: Regularization, cross-validation, appropriate model complexity

### Data Quality Issues

- Incomplete or biased datasets
- Measurement errors
- Inconsistent data formats
- Privacy and ethical concerns

### Interpretability vs. Performance

Complex models (deep neural networks) often perform better but are harder to interpret, while simpler models are more transparent but may have limited capability.

## Real-World Applications

### Healthcare

- Medical image analysis for cancer detection
- Drug discovery and development
- Personalized treatment recommendations
- Epidemic modeling and prediction

### Finance

- Algorithmic trading
- Fraud detection
- Credit risk assessment
- Portfolio optimization

### Technology

- Search engines and information retrieval
- Natural language processing and translation
- Computer vision and autonomous systems
- Recommendation engines

### Transportation

- Route optimization
- Autonomous vehicles
- Traffic management
- Predictive maintenance

## Future Directions

### Emerging Trends

- **Federated Learning**: Training models across decentralized data
- **Explainable AI**: Making complex models more interpretable
- **AutoML**: Automating the machine learning pipeline
- **Edge Computing**: Running ML models on local devices
- **Quantum Machine Learning**: Leveraging quantum computing for ML

### Ethical Considerations

- Bias and fairness in algorithms
- Privacy protection and data security
- Transparency and accountability
- Impact on employment and society

## Getting Started

For those interested in learning machine learning:

1. **Foundation**: Statistics, linear algebra, programming (Python/R)
2. **Tools**: Scikit-learn, TensorFlow, PyTorch, Jupyter notebooks
3. **Practice**: Kaggle competitions, personal projects, online courses
4. **Community**: Join ML communities, attend conferences, collaborate on open-source projects

Machine learning continues to evolve rapidly, offering exciting opportunities to solve complex problems and create innovative solutions across diverse fields.
            "#.trim().to_string(),
            "text/markdown".to_string(),
        );
        
        doc.id = "machine-learning-fundamentals-001".to_string();
        doc.summary = Some("Comprehensive introduction to machine learning concepts, algorithms, applications, and best practices.".to_string());
        doc.author = Some("AI Research Consortium".to_string());
        doc.category = Some("Technology".to_string());
        doc.set_tags(vec!["machine learning".to_string(), "artificial intelligence".to_string(), "algorithms".to_string(), "data science".to_string(), "technology".to_string()]);
        doc.reading_time = Some(15);
        doc.difficulty_level = Some(3);
        doc.language = "en".to_string();
        
        doc
    }
    
    /// Get total word count of sample documents
    pub fn get_sample_word_count() -> usize {
        let documents = Self::get_sample_documents();
        documents.iter()
            .map(|doc| doc.content.split_whitespace().count())
            .sum()
    }
}
