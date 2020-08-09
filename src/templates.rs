pub const SHELL_WELCOME_MSG: &str = "# Hello, scripter!
# Here are some usefule commands to begin with:

cd {{birthplace}}
";

pub const JS_WELCOME_MSG: &str = "// Hello, scripter!
// Here are some information you may be intrested in:

const birthplace = \"{{birthplace}}\"
";

pub const SCREEN_WELCOME_MSG: &str = "layout new
screen bash -c \"cd {{birthplace}};\"
";
