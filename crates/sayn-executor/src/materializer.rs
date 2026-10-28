use sayn_ir::GenerationPlan;
use sayn_security::safe_write_file;
use std::fs;
use std::path::Path;

pub struct Materializer;

impl Materializer {
    pub fn materialize_plan(
        gen_root: &Path,
        plan: &GenerationPlan,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        fs::create_dir_all(gen_root)?;
        let mut written_files = 0;

        // 1. package.json
        written_files += Self::write_package_json(gen_root, plan)?;

        // 2. tsconfig.json
        written_files += Self::write_tsconfig(gen_root)?;

        // 3. next.config.ts
        written_files += Self::write_next_config(gen_root)?;

        // 4. postcss.config.mjs
        written_files += Self::write_postcss_config(gen_root)?;

        // 5. app/globals.css (Tailwind CSS v4)
        written_files += Self::write_globals_css(gen_root)?;

        // 6. app/layout.tsx
        written_files += Self::write_layout(gen_root, plan)?;

        // 7. app/page.tsx (home page)
        written_files += Self::write_home_page(gen_root, plan)?;

        // 8. Additional route pages
        for route in &plan.routes {
            if route.route == "/" {
                continue; // Already handled by write_home_page
            }
            written_files += Self::write_route_page(gen_root, plan, route)?;
        }

        // 9. Components
        for comp in &plan.components {
            written_files += Self::write_component(gen_root, comp)?;
        }

        // 10. .gitignore for generated project
        written_files += Self::write_gen_gitignore(gen_root)?;

        Ok(written_files)
    }

    fn write_package_json(
        gen_root: &Path,
        plan: &GenerationPlan,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let mut deps_obj = serde_json::Map::new();
        let mut dev_deps_obj = serde_json::Map::new();

        for dep in &plan.dependencies {
            let val = serde_json::Value::String(dep.version.clone());
            if dep.is_dev {
                dev_deps_obj.insert(dep.name.clone(), val);
            } else {
                deps_obj.insert(dep.name.clone(), val);
            }
        }

        let package_json = serde_json::json!({
            "name": plan.project_name,
            "version": "0.1.0",
            "private": true,
            "scripts": {
                "dev": "next dev --turbopack",
                "build": "next build",
                "start": "next start",
                "lint": "next lint"
            },
            "dependencies": deps_obj,
            "devDependencies": dev_deps_obj
        });

        let pkg_content = serde_json::to_string_pretty(&package_json)?;
        safe_write_file(gen_root, Path::new("package.json"), pkg_content.as_bytes())?;
        Ok(1)
    }

    fn write_tsconfig(gen_root: &Path) -> Result<usize, Box<dyn std::error::Error>> {
        let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2017",
    "lib": ["dom", "dom.iterable", "esnext"],
    "allowJs": true,
    "skipLibCheck": true,
    "strict": true,
    "noEmit": true,
    "esModuleInterop": true,
    "module": "esnext",
    "moduleResolution": "bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "jsx": "preserve",
    "incremental": true,
    "plugins": [{ "name": "next" }],
    "paths": {
      "@/*": ["./*"]
    }
  },
  "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx", ".next/types/**/*.ts"],
  "exclude": ["node_modules"]
}
"#;
        safe_write_file(gen_root, Path::new("tsconfig.json"), tsconfig.as_bytes())?;
        Ok(1)
    }

    fn write_next_config(gen_root: &Path) -> Result<usize, Box<dyn std::error::Error>> {
        let next_config = r#"import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  /* config options here */
};

export default nextConfig;
"#;
        safe_write_file(gen_root, Path::new("next.config.ts"), next_config.as_bytes())?;
        Ok(1)
    }

    fn write_postcss_config(gen_root: &Path) -> Result<usize, Box<dyn std::error::Error>> {
        let postcss = r#"/** @type {import('postcss-load-config').Config} */
const config = {
  plugins: {
    "@tailwindcss/postcss": {},
  },
};

export default config;
"#;
        safe_write_file(gen_root, Path::new("postcss.config.mjs"), postcss.as_bytes())?;
        Ok(1)
    }

    fn write_globals_css(gen_root: &Path) -> Result<usize, Box<dyn std::error::Error>> {
        let globals_css = r#"@import "tailwindcss";
"#;
        safe_write_file(gen_root, Path::new("app/globals.css"), globals_css.as_bytes())?;
        Ok(1)
    }

    fn write_layout(
        gen_root: &Path,
        plan: &GenerationPlan,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let layout_tsx = format!(
            r#"import type {{ Metadata }} from "next";
import {{ Geist, Geist_Mono }} from "next/font/google";
import "./globals.css";

const geistSans = Geist({{
  variable: "--font-geist-sans",
  subsets: ["latin"],
}});

const geistMono = Geist_Mono({{
  variable: "--font-geist-mono",
  subsets: ["latin"],
}});

export const metadata: Metadata = {{
  title: "{name}",
  description: "Generated by Sayn",
}};

export default function RootLayout({{
  children,
}}: Readonly<{{
  children: React.ReactNode;
}}>) {{
  return (
    <html lang="en">
      <body
        className={{`${{geistSans.variable}} ${{geistMono.variable}} antialiased`}}
      >
        {{children}}
      </body>
    </html>
  );
}}
"#,
            name = plan.project_name
        );
        safe_write_file(gen_root, Path::new("app/layout.tsx"), layout_tsx.as_bytes())?;
        Ok(1)
    }

    fn write_home_page(
        gen_root: &Path,
        plan: &GenerationPlan,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        // Collect components for the home page
        let mut component_imports = String::new();
        let mut component_elements = String::new();

        for comp in &plan.components {
            let import_path = comp
                .file_path
                .with_extension("")
                .display()
                .to_string()
                .replace('\\', "/");
            component_imports.push_str(&format!(
                "import {{ {} }} from \"@/{}\";\n",
                comp.name, import_path
            ));
            component_elements.push_str(&format!("        <{name} />\n", name = comp.name));
        }

        let page_tsx = format!(
            r#"import Image from "next/image";
{imports}
export default function Home() {{
  return (
    <div className="grid grid-rows-[20px_1fr_20px] items-center justify-items-center min-h-screen p-8 pb-20 gap-16 sm:p-20 font-[family-name:var(--font-geist-sans)]">
      <main className="flex flex-col gap-[32px] row-start-2 items-center sm:items-start">
{elements}      </main>
    </div>
  );
}}
"#,
            imports = component_imports,
            elements = if component_elements.is_empty() {
                "        <h1 className=\"text-4xl font-bold\">Welcome to Sayn</h1>\n".to_string()
            } else {
                component_elements
            }
        );
        safe_write_file(gen_root, Path::new("app/page.tsx"), page_tsx.as_bytes())?;
        Ok(1)
    }

    fn write_route_page(
        gen_root: &Path,
        _plan: &GenerationPlan,
        route: &sayn_ir::RoutePlan,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let route_name = route
            .route
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or("page");

        // Convert route name to a display title
        let title = route_name
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i == 0 {
                    c.to_uppercase().next().unwrap_or(c)
                } else {
                    c
                }
            })
            .collect::<String>();

        let page_tsx = format!(
            r#"export default function {title}Page() {{
  return (
    <div className="grid grid-rows-[20px_1fr_20px] items-center justify-items-center min-h-screen p-8 pb-20 gap-16 sm:p-20 font-[family-name:var(--font-geist-sans)]">
      <main className="flex flex-col gap-8 row-start-2 items-center sm:items-start">
        <h1 className="text-4xl font-bold">{title}</h1>
        <p className="text-lg text-gray-600 dark:text-gray-400">
          This page was generated from your .sayn instructions.
        </p>
      </main>
    </div>
  );
}}
"#,
            title = title
        );

        safe_write_file(gen_root, &route.target_file, page_tsx.as_bytes())?;
        Ok(1)
    }

    fn write_component(
        gen_root: &Path,
        comp: &sayn_ir::ComponentPlan,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let comp_code = format!(
            r#"export function {name}() {{
  return (
    <section className="w-full py-12 px-6">
      <div className="max-w-5xl mx-auto">
        <h2 className="text-2xl font-bold mb-4">{name}</h2>
        <p className="text-gray-600 dark:text-gray-400">{desc}</p>
      </div>
    </section>
  );
}}
"#,
            name = comp.name,
            desc = comp.description
        );
        safe_write_file(gen_root, &comp.file_path, comp_code.as_bytes())?;
        Ok(1)
    }

    fn write_gen_gitignore(gen_root: &Path) -> Result<usize, Box<dyn std::error::Error>> {
        let gitignore = r#"# dependencies
/node_modules
/.pnp
.pnp.*
.yarn/*
!.yarn/patches
!.yarn/plugins
!.yarn/releases
!.yarn/versions

# testing
/coverage

# next.js
/.next/
/out/

# production
/build

# misc
.DS_Store
*.pem

# debug
npm-debug.log*
yarn-debug.log*
yarn-error.log*
.pnpm-debug.log*

# env files
.env*.local

# vercel
.vercel

# typescript
*.tsbuildinfo
next-env.d.ts
"#;
        safe_write_file(gen_root, Path::new(".gitignore"), gitignore.as_bytes())?;
        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sayn_core::{Framework, Target};
    use sayn_ir::{ComponentPlan, DependencyPlan, RoutePlan, TokenEstimate};
    use tempfile::tempdir;

    #[test]
    fn test_materialize_plan_basic() {
        let dir = tempdir().unwrap();
        let gen_root = dir.path().join("generated");

        let plan = GenerationPlan {
            project_name: "test-app".into(),
            target: Target::Web,
            framework: Framework::NextJs,
            instructions_count: 1,
            routes: vec![
                RoutePlan {
                    route: "/".into(),
                    source_instruction: "app/page.sayn".into(),
                    target_file: "app/page.tsx".into(),
                },
            ],
            components: vec![ComponentPlan {
                name: "Navbar".into(),
                description: "Top navigation".into(),
                file_path: "components/Navbar.tsx".into(),
            }],
            dependencies: vec![
                DependencyPlan {
                    name: "next".into(),
                    version: "15.3.1".into(),
                    is_dev: false,
                },
                DependencyPlan {
                    name: "react".into(),
                    version: "^19.0.0".into(),
                    is_dev: false,
                },
                DependencyPlan {
                    name: "react-dom".into(),
                    version: "^19.0.0".into(),
                    is_dev: false,
                },
            ],
            operations: vec![],
            token_estimate: TokenEstimate::default(),
            warnings: vec![],
        };

        let count = Materializer::materialize_plan(&gen_root, &plan).unwrap();
        // package.json, tsconfig, next.config, postcss, globals.css, layout, page, Navbar, .gitignore = 9
        assert!(count >= 9, "Expected at least 9 files, got {}", count);
        assert!(gen_root.join("package.json").exists());
        assert!(gen_root.join("tsconfig.json").exists());
        assert!(gen_root.join("next.config.ts").exists());
        assert!(gen_root.join("postcss.config.mjs").exists());
        assert!(gen_root.join("app/globals.css").exists());
        assert!(gen_root.join("app/layout.tsx").exists());
        assert!(gen_root.join("app/page.tsx").exists());
        assert!(gen_root.join("components/Navbar.tsx").exists());
        assert!(gen_root.join(".gitignore").exists());
    }

    #[test]
    fn test_materialize_multi_route() {
        let dir = tempdir().unwrap();
        let gen_root = dir.path().join("generated");

        let plan = GenerationPlan {
            project_name: "multi-page".into(),
            target: Target::Web,
            framework: Framework::NextJs,
            instructions_count: 2,
            routes: vec![
                RoutePlan {
                    route: "/".into(),
                    source_instruction: "app/page.sayn".into(),
                    target_file: "app/page.tsx".into(),
                },
                RoutePlan {
                    route: "/about".into(),
                    source_instruction: "app/about.sayn".into(),
                    target_file: "app/about/page.tsx".into(),
                },
                RoutePlan {
                    route: "/projects".into(),
                    source_instruction: "app/projects.sayn".into(),
                    target_file: "app/projects/page.tsx".into(),
                },
            ],
            components: vec![],
            dependencies: vec![DependencyPlan {
                name: "next".into(),
                version: "15.3.1".into(),
                is_dev: false,
            }],
            operations: vec![],
            token_estimate: TokenEstimate::default(),
            warnings: vec![],
        };

        let count = Materializer::materialize_plan(&gen_root, &plan).unwrap();
        assert!(gen_root.join("app/page.tsx").exists());
        assert!(gen_root.join("app/about/page.tsx").exists());
        assert!(gen_root.join("app/projects/page.tsx").exists());
        assert!(count >= 10, "Expected at least 10 files, got {}", count);
    }

    #[test]
    fn test_globals_css_contains_tailwind() {
        let dir = tempdir().unwrap();
        let gen_root = dir.path().join("generated");

        let plan = GenerationPlan {
            project_name: "tw-test".into(),
            target: Target::Web,
            framework: Framework::NextJs,
            instructions_count: 1,
            routes: vec![],
            components: vec![],
            dependencies: vec![],
            operations: vec![],
            token_estimate: TokenEstimate::default(),
            warnings: vec![],
        };

        Materializer::materialize_plan(&gen_root, &plan).unwrap();
        let css = std::fs::read_to_string(gen_root.join("app/globals.css")).unwrap();
        assert!(css.contains("tailwindcss"));
    }
}
