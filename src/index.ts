import fs from "node:fs";
import path from "node:path";

type PrimitiveTypeName = "int" | "float" | "bool" | "str" | "void" | "Component";

type TypeRef =
  | { kind: "primitive"; name: PrimitiveTypeName }
  | { kind: "named"; name: string }
  | { kind: "tuple"; members: TypeRef[] }
  | { kind: "self" };

type FunctionArg = { name: string; type: TypeRef; isSelf?: boolean };

type ImportItem = { name: string; alias?: string };

type ImportDecl = {
  kind: "import";
  path: string[];
  alias?: string;
  items?: ImportItem[];
  isPublic: boolean;
};

type AliasDecl = {
  kind: "alias";
  name: string;
  target: TypeRef;
  isPublic: boolean;
};

type ObjectField = { name: string; type: TypeRef };

type FunctionBody =
  | { kind: "arrow"; expression: Expr }
  | { kind: "block"; statements: Statement[]; tail?: Expr };

type FunctionDecl = {
  kind: "function";
  name: string;
  args: FunctionArg[];
  returnType: TypeRef;
  body: FunctionBody;
  isPublic: boolean;
};

type ObjectDecl = {
  kind: "object";
  name: string;
  fields: ObjectField[];
  methods: FunctionDecl[];
  isPublic: boolean;
};

type ComponentProp = {
  name: string;
  type?: TypeRef;
  defaultValue?: Expr;
  isPublic: boolean;
  isChildren?: boolean;
};

type ComponentItem =
  | { kind: "prop"; prop: ComponentProp }
  | { kind: "child"; expression: Expr }
  | { kind: "field"; name: string; expression: Expr };

type ComponentDecl = {
  kind: "component";
  name: string;
  items: ComponentItem[];
  isPublic: boolean;
};

type StylesheetDecl = {
  kind: "stylesheet";
  name: string;
  params: FunctionArg[];
  uses: Expr[];
  isPublic: boolean;
};

type Declaration = ImportDecl | AliasDecl | FunctionDecl | ObjectDecl | ComponentDecl | StylesheetDecl;

type LetStatement = {
  kind: "let";
  name: string;
  mutable: boolean;
  declaredType?: TypeRef;
  value?: Expr;
};

type AssignmentStatement = {
  kind: "assignment";
  target: Expr;
  value: Expr;
};

type WhileStatement = {
  kind: "while";
  condition: Expr;
  body: Statement[];
};

type ExprStatement = {
  kind: "expr";
  expression: Expr;
};

type Statement = LetStatement | AssignmentStatement | WhileStatement | ExprStatement;

type Expr =
  | { kind: "int"; raw: string; value: number }
  | { kind: "float"; raw: string; value: number }
  | { kind: "string"; value: string }
  | { kind: "bool"; value: boolean }
  | { kind: "identifier"; name: string }
  | { kind: "binary"; op: string; left: Expr; right: Expr }
  | { kind: "call"; callee: Expr; args: Expr[] }
  | { kind: "namedCall"; callee: Expr; args: { name: string; value: Expr }[] }
  | { kind: "component"; callee: Expr; entries: ComponentEntry[] }
  | { kind: "field"; object: Expr; field: string }
  | { kind: "tupleAccess"; object: Expr; index: number }
  | { kind: "tuple"; members: Expr[] }
  | { kind: "if"; condition: Expr; thenBranch: BlockExpr; elseBranch: BlockExpr }
  | { kind: "group"; expression: Expr };

type ComponentEntry =
  | { kind: "prop"; name: string; expression: Expr }
  | { kind: "child"; expression: Expr };

type BlockExpr = { statements: Statement[]; tail?: Expr };

type Token = {
  type: "identifier" | "number" | "string" | "symbol" | "operator" | "eof";
  value: string;
  index: number;
};

type SourceModule = {
  filePath: string;
  declarations: Declaration[];
};

type DeclRef =
  | { kind: "function"; module: SourceModule; declaration: FunctionDecl }
  | { kind: "object"; module: SourceModule; declaration: ObjectDecl }
  | { kind: "component"; module: SourceModule; declaration: ComponentDecl }
  | { kind: "alias"; module: SourceModule; declaration: AliasDecl }
  | { kind: "stylesheet"; module: SourceModule; declaration: StylesheetDecl };

type Env = {
  vars: Map<string, TypeRef>;
  objectSelf?: string;
};

const primitiveTypes = new Set<PrimitiveTypeName>(["int", "float", "bool", "str", "void", "Component"]);
const builtinComponents = new Set(["Div", "Text", "Icon"]);

class SlynxError extends Error {
  constructor(message: string, readonly kind: string) {
    super(`${kind}: ${message}`);
    this.name = "SlynxError";
  }
}

class Lexer {
  static tokenize(source: string): Token[] {
    const tokens: Token[] = [];
    let index = 0;

    const push = (type: Token["type"], value: string, start: number) => {
      tokens.push({ type, value, index: start });
    };

    while (index < source.length) {
      const char = source[index];

      if (/\s/.test(char)) {
        index += 1;
        continue;
      }

      if (char === "/" && source[index + 1] === "/") {
        index += 2;
        while (index < source.length && source[index] !== "\n") {
          index += 1;
        }
        continue;
      }

      if (char === "/" && source[index + 1] === "*") {
        index += 2;
        while (index + 1 < source.length && !(source[index] === "*" && source[index + 1] === "/")) {
          index += 1;
        }
        if (index + 1 >= source.length) {
          throw new SlynxError("Unterminated block comment", "LexerError");
        }
        index += 2;
        continue;
      }

      if (char === "@") {
        push("symbol", "@", index);
        index += 1;
        continue;
      }

      if (char === "\"" || char === "'") {
        const quote = char;
        const start = index;
        index += 1;
        let value = "";
        while (index < source.length && source[index] !== quote) {
          if (source[index] === "\\" && index + 1 < source.length) {
            value += source[index + 1];
            index += 2;
            continue;
          }
          value += source[index];
          index += 1;
        }
        if (source[index] !== quote) {
          throw new SlynxError("Unterminated string literal", "LexerError");
        }
        index += 1;
        push("string", value, start);
        continue;
      }

      if (/[0-9]/.test(char)) {
        const start = index;
        if (char === "0" && ["x", "X", "b", "B", "o", "O"].includes(source[index + 1] ?? "")) {
          const radix = source[index + 1];
          index += 2;
          const pattern =
            radix === "x" || radix === "X"
              ? /[0-9A-Fa-f_]/
              : radix === "b" || radix === "B"
                ? /[01_]/
                : /[0-7_]/;
          while (index < source.length && pattern.test(source[index])) {
            index += 1;
          }
        } else {
          index += 1;
          while (index < source.length && /[0-9_]/.test(source[index])) {
            index += 1;
          }
          if (source[index] === "." && /[0-9]/.test(source[index + 1] ?? "")) {
            index += 1;
            while (index < source.length && /[0-9_]/.test(source[index])) {
              index += 1;
            }
          }
        }
        push("number", source.slice(start, index), start);
        continue;
      }

      if (/[A-Za-z_]/.test(char)) {
        const start = index;
        index += 1;
        while (index < source.length && /[A-Za-z0-9_]/.test(source[index])) {
          index += 1;
        }
        push("identifier", source.slice(start, index), start);
        continue;
      }

      const three = source.slice(index, index + 3);
      const two = source.slice(index, index + 2);
      if (["->"].includes(two)) {
        push("operator", two, index);
        index += 2;
        continue;
      }
      if (["==", "<=", ">=", "&&", "||", "<<", ">>"].includes(two)) {
        push("operator", two, index);
        index += 2;
        continue;
      }
      if ("{}()[],:;.=+-*/<>|&^".includes(char)) {
        const type = "=+-*/<>|&^".includes(char) ? "operator" : "symbol";
        push(type, char, index);
        index += 1;
        continue;
      }

      throw new SlynxError(`Unexpected character '${char}'`, "LexerError");
    }

    tokens.push({ type: "eof", value: "", index: source.length });
    return tokens;
  }
}

class Parser {
  private index = 0;

  constructor(private readonly tokens: Token[]) {}

  parseDeclarations(): Declaration[] {
    const declarations: Declaration[] = [];
    while (!this.is("eof")) {
      declarations.push(this.parseDeclaration());
    }
    return declarations;
  }

  private parseDeclaration(): Declaration {
    this.skipAttributes();
    const isPublic = this.matchIdentifier("pub");

    if (this.matchIdentifier("import")) {
      const decl = this.parseImport();
      decl.isPublic = isPublic;
      return decl;
    }
    if (this.matchIdentifier("alias")) {
      return this.parseAlias(isPublic);
    }
    if (this.matchIdentifier("func")) {
      return this.parseFunction(isPublic);
    }
    if (this.matchIdentifier("object")) {
      return this.parseObject(isPublic);
    }
    if (this.matchIdentifier("component")) {
      return this.parseComponent(isPublic);
    }
    if (this.matchIdentifier("stylesheet")) {
      return this.parseStylesheet(isPublic);
    }

    throw this.error(`Unexpected top-level token '${this.peek().value}'`);
  }

  private parseImport(): ImportDecl {
    const parts = [this.expectIdentifier()];
    while (this.matchSymbol(".")) {
      parts.push(this.expectIdentifier());
    }

    let alias: string | undefined;
    let items: ImportItem[] | undefined;

    if (this.matchIdentifier("using")) {
      if (this.matchSymbol("{")) {
        items = [];
        while (!this.matchSymbol("}")) {
          const name = this.expectIdentifier();
          let itemAlias: string | undefined;
          if (this.matchIdentifier("as")) {
            itemAlias = this.expectIdentifier();
          }
          items.push({ name, alias: itemAlias });
          this.matchSymbol(",");
        }
      } else if (this.peek().type === "identifier" && this.tokens[this.index + 1]?.value === ";") {
        items = [{ name: this.expectIdentifier() }];
      } else {
        alias = this.expectIdentifier();
        this.expectIdentifier("as");
        const renamed = this.expectIdentifier();
        items = [{ name: alias, alias: renamed }];
        alias = undefined;
      }
    }

    this.expectSymbol(";");
    return { kind: "import", path: parts, alias, items, isPublic: false };
  }

  private parseAlias(isPublic: boolean): AliasDecl {
    const name = this.expectIdentifier();
    this.expectOperator("=");
    const target = this.parseTypeRef();
    this.expectSymbol(";");
    return { kind: "alias", name, target, isPublic };
  }

  private parseFunction(isPublic: boolean): FunctionDecl {
    const name = this.expectIdentifier();
    this.expectSymbol("(");
    const args: FunctionArg[] = [];
    while (!this.matchSymbol(")")) {
      const argName = this.expectIdentifier();
      let arg: FunctionArg;
      if (this.matchSymbol(":")) {
        arg = { name: argName, type: this.parseTypeRef() };
      } else if (argName === "self") {
        arg = { name: argName, type: { kind: "self" }, isSelf: true };
      } else {
        throw this.error(`Expected ':' after argument '${argName}'`);
      }
      args.push(arg);
      this.matchSymbol(",");
    }
    this.expectSymbol(":");
    const returnType = this.parseTypeRef();

    let body: FunctionBody;
    if (this.matchOperator("->")) {
      const expression = this.parseExpression();
      this.expectSymbol(";");
      body = { kind: "arrow", expression };
    } else {
      body = { kind: "block", ...this.parseBlockBody() };
    }
    return { kind: "function", name, args, returnType, body, isPublic };
  }

  private parseObject(isPublic: boolean): ObjectDecl {
    const name = this.expectIdentifier();
    this.expectSymbol("{");
    const fields: ObjectField[] = [];
    const methods: FunctionDecl[] = [];
    while (!this.matchSymbol("}")) {
      this.skipAttributes();
      const memberPublic = this.matchIdentifier("pub");
      if (this.matchIdentifier("func")) {
        methods.push(this.parseFunction(memberPublic));
        continue;
      }
      const fieldName = this.expectIdentifier();
      this.expectSymbol(":");
      fields.push({ name: fieldName, type: this.parseTypeRef() });
      this.matchSymbol(",");
    }
    return { kind: "object", name, fields, methods, isPublic };
  }

  private parseComponent(isPublic: boolean): ComponentDecl {
    const name = this.expectIdentifier();
    this.expectSymbol("{");
    const items: ComponentItem[] = [];
    while (!this.matchSymbol("}")) {
      this.skipAttributes();
      const memberPublic = this.matchIdentifier("pub");
      if (this.matchIdentifier("prop")) {
        const propName = this.expectIdentifier();
        let propType: TypeRef | undefined;
        let defaultValue: Expr | undefined;
        if (this.matchSymbol(":")) {
          propType = this.parseTypeRef();
        }
        if (this.matchOperator("=")) {
          defaultValue = this.parseExpression();
        }
        this.expectSymbol(";");
        items.push({
          kind: "prop",
          prop: {
            name: propName,
            type: propType,
            defaultValue,
            isPublic: memberPublic,
            isChildren: propName === "children"
          }
        });
        continue;
      }

      const expression = this.parseExpression();
      if (expression.kind === "component") {
        items.push({ kind: "child", expression });
      } else if (expression.kind === "binary" && expression.op === ":") {
        throw this.error("Invalid component field entry");
      } else {
        items.push({ kind: "child", expression });
      }
      this.matchSymbol(",");
    }
    return { kind: "component", name, items, isPublic };
  }

  private parseStylesheet(isPublic: boolean): StylesheetDecl {
    const name = this.expectIdentifier();
    this.expectSymbol("(");
    const params: FunctionArg[] = [];
    while (!this.matchSymbol(")")) {
      const paramName = this.expectIdentifier();
      this.expectSymbol(":");
      params.push({ name: paramName, type: this.parseTypeRef() });
      this.matchSymbol(",");
    }
    const uses: Expr[] = [];
    if (this.matchIdentifier("uses")) {
      uses.push(this.parseExpression());
      while (this.matchSymbol(",")) {
        uses.push(this.parseExpression());
      }
    }
    this.expectSymbol("{");
    let depth = 1;
    while (depth > 0) {
      const token = this.next();
      if (token.type === "eof") {
        throw this.error("Unterminated stylesheet body");
      }
      if (token.value === "{") {
        depth += 1;
      } else if (token.value === "}") {
        depth -= 1;
      }
    }
    return { kind: "stylesheet", name, params, uses, isPublic };
  }

  private parseBlockBody(): BlockExpr {
    this.expectSymbol("{");
    const statements: Statement[] = [];
    let tail: Expr | undefined;
    while (!this.matchSymbol("}")) {
      if (this.checkIdentifier("let")) {
        statements.push(this.parseLetStatement());
        continue;
      }
      if (this.checkIdentifier("while")) {
        statements.push(this.parseWhileStatement());
        continue;
      }

      const expr = this.parseExpression();
      if (this.matchOperator("=")) {
        const value = this.parseExpression();
        this.expectSymbol(";");
        statements.push({ kind: "assignment", target: expr, value });
        continue;
      }
      if (this.matchSymbol(";")) {
        statements.push({ kind: "expr", expression: expr });
        continue;
      }
      tail = expr;
      this.expectSymbol("}");
      break;
    }
    return { statements, tail };
  }

  private parseLetStatement(): LetStatement {
    this.expectIdentifier("let");
    const mutable = this.matchIdentifier("mut");
    const name = this.expectIdentifier();
    let declaredType: TypeRef | undefined;
    let value: Expr | undefined;
    if (this.matchSymbol(":")) {
      declaredType = this.parseTypeRef();
    }
    if (this.matchOperator("=")) {
      value = this.parseExpression();
    }
    this.expectSymbol(";");
    return { kind: "let", name, mutable, declaredType, value };
  }

  private parseWhileStatement(): WhileStatement {
    this.expectIdentifier("while");
    const condition = this.parseExpression();
    const body = this.parseBlockBody().statements;
    return { kind: "while", condition, body };
  }

  private parseTypeRef(): TypeRef {
    if (this.matchSymbol("(")) {
      const members: TypeRef[] = [];
      if (!this.matchSymbol(")")) {
        do {
          members.push(this.parseTypeRef());
        } while (this.matchSymbol(","));
        this.expectSymbol(")");
      }
      return { kind: "tuple", members };
    }
    const name = this.expectIdentifier();
    if (name === "Self") {
      return { kind: "self" };
    }
    if (primitiveTypes.has(name as PrimitiveTypeName)) {
      return { kind: "primitive", name: name as PrimitiveTypeName };
    }
    return { kind: "named", name };
  }

  private parseExpression(minPrecedence = 0, options: { allowComponent?: boolean } = {}): Expr {
    const allowComponent = options.allowComponent ?? true;
    let left = this.parsePrimary();

    while (true) {
      if (this.matchSymbol(".")) {
        const next = this.peek();
        if (next.type === "number") {
          left = { kind: "tupleAccess", object: left, index: Number.parseInt(this.next().value, 10) };
        } else {
          left = { kind: "field", object: left, field: this.expectIdentifier() };
        }
        continue;
      }

      if (this.checkSymbol("(")) {
        if (this.looksLikeNamedCall()) {
          this.expectSymbol("(");
          const args: { name: string; value: Expr }[] = [];
          while (!this.matchSymbol(")")) {
            const name = this.expectIdentifier();
            this.expectSymbol(":");
            args.push({ name, value: this.parseExpression() });
            this.matchSymbol(",");
          }
          left = { kind: "namedCall", callee: left, args };
          continue;
        }

        this.expectSymbol("(");
        const args: Expr[] = [];
        while (!this.matchSymbol(")")) {
          args.push(this.parseExpression());
          this.matchSymbol(",");
        }
        left = { kind: "call", callee: left, args };
        continue;
      }

      if (allowComponent && this.checkSymbol("{") && this.isComponentCallee(left)) {
        this.expectSymbol("{");
        const entries: ComponentEntry[] = [];
        while (!this.matchSymbol("}")) {
          if (this.peek().type === "identifier" && this.tokens[this.index + 1]?.value === ":") {
            const name = this.expectIdentifier();
            this.expectSymbol(":");
            entries.push({ kind: "prop", name, expression: this.parseExpression(0, options) });
          } else {
            entries.push({ kind: "child", expression: this.parseExpression(0, options) });
          }
          this.matchSymbol(",");
        }
        left = { kind: "component", callee: left, entries };
        continue;
      }

      const operator = this.peek();
      const precedence = binaryPrecedence(operator.value);
      if (precedence < minPrecedence) {
        break;
      }
      this.next();
      const right = this.parseExpression(precedence + 1, options);
      left = { kind: "binary", op: operator.value, left, right };
    }

    return left;
  }

  private parsePrimary(): Expr {
    const token = this.next();

    if (token.type === "number") {
      if (token.value.includes(".")) {
        return { kind: "float", raw: token.value, value: parseNumber(token.value) };
      }
      return { kind: "int", raw: token.value, value: parseNumber(token.value) };
    }
    if (token.type === "string") {
      return { kind: "string", value: token.value };
    }
    if (token.type === "identifier") {
      if (token.value === "true" || token.value === "false") {
        return { kind: "bool", value: token.value === "true" };
      }
      if (token.value === "if") {
        const condition = this.parseExpression(0, { allowComponent: false });
        const thenBranch = this.parseBlockBody();
        this.expectIdentifier("else");
        const elseBranch = this.parseBlockBody();
        return { kind: "if", condition, thenBranch, elseBranch };
      }
      return { kind: "identifier", name: token.value };
    }
    if (token.value === "(") {
      const first = this.parseExpression();
      if (this.matchSymbol(",")) {
        const members = [first];
        do {
          members.push(this.parseExpression());
        } while (this.matchSymbol(","));
        this.expectSymbol(")");
        return { kind: "tuple", members };
      }
      this.expectSymbol(")");
      return { kind: "group", expression: first };
    }

    throw this.error(`Unexpected expression token '${token.value}'`);
  }

  private skipAttributes(): void {
    while (this.matchSymbol("@")) {
      this.expectIdentifier();
      if (this.matchSymbol("(")) {
        let depth = 1;
        while (depth > 0) {
          const token = this.next();
          if (token.type === "eof") {
            throw this.error("Unterminated attribute");
          }
          if (token.value === "(") depth += 1;
          if (token.value === ")") depth -= 1;
        }
      }
    }
  }

  private is(type: Token["type"]): boolean {
    return this.peek().type === type;
  }

  private peek(): Token {
    return this.tokens[this.index];
  }

  private next(): Token {
    return this.tokens[this.index++];
  }

  private matchIdentifier(value: string): boolean {
    if (this.peek().type === "identifier" && this.peek().value === value) {
      this.index += 1;
      return true;
    }
    return false;
  }

  private checkIdentifier(value: string): boolean {
    return this.peek().type === "identifier" && this.peek().value === value;
  }

  private expectIdentifier(expected?: string): string {
    const token = this.next();
    if (token.type !== "identifier") {
      throw this.error(`Expected identifier, got '${token.value}'`);
    }
    if (expected && token.value !== expected) {
      throw this.error(`Expected identifier '${expected}', got '${token.value}'`);
    }
    return token.value;
  }

  private matchSymbol(value: string): boolean {
    if (this.peek().value === value) {
      this.index += 1;
      return true;
    }
    return false;
  }

  private checkSymbol(value: string): boolean {
    return this.peek().value === value;
  }

  private expectSymbol(value: string): void {
    const token = this.next();
    if (token.value !== value) {
      throw this.error(`Expected symbol '${value}', got '${token.value}'`);
    }
  }

  private matchOperator(value: string): boolean {
    if (this.peek().type === "operator" && this.peek().value === value) {
      this.index += 1;
      return true;
    }
    return false;
  }

  private expectOperator(value: string): void {
    const token = this.next();
    if (token.type !== "operator" || token.value !== value) {
      throw this.error(`Expected operator '${value}', got '${token.value}'`);
    }
  }

  private looksLikeNamedCall(): boolean {
    return (
      this.peek().value === "(" &&
      this.tokens[this.index + 1]?.type === "identifier" &&
      this.tokens[this.index + 2]?.value === ":"
    );
  }

  private isComponentCallee(expr: Expr): boolean {
    return expr.kind === "identifier" || expr.kind === "field";
  }

  private error(message: string): SlynxError {
    return new SlynxError(message, "ParserError");
  }
}

function binaryPrecedence(op: string): number {
  switch (op) {
    case "||":
      return 1;
    case "&&":
      return 2;
    case "==":
    case "<":
    case "<=":
    case ">":
    case ">=":
      return 3;
    case "|":
      return 4;
    case "^":
      return 5;
    case "&":
      return 6;
    case "<<":
    case ">>":
      return 7;
    case "+":
    case "-":
      return 8;
    case "*":
    case "/":
      return 9;
    default:
      return -1;
  }
}

function parseNumber(raw: string): number {
  const normalized = raw.replaceAll("_", "");
  if (normalized.startsWith("0x") || normalized.startsWith("0X")) {
    return Number.parseInt(normalized.slice(2), 16);
  }
  if (normalized.startsWith("0b") || normalized.startsWith("0B")) {
    return Number.parseInt(normalized.slice(2), 2);
  }
  if (normalized.startsWith("0o") || normalized.startsWith("0O")) {
    return Number.parseInt(normalized.slice(2), 8);
  }
  return Number(normalized);
}

class ModuleRegistry {
  private readonly modules = new Map<string, SourceModule>();

  constructor(private readonly stdPath?: string) {}

  load(entryPath: string): SourceModule {
    const resolved = path.resolve(entryPath);
    return this.loadFile(resolved);
  }

  allModules(): SourceModule[] {
    return [...this.modules.values()];
  }

  private loadFile(filePath: string): SourceModule {
    const resolved = path.resolve(filePath);
    const cached = this.modules.get(resolved);
    if (cached) {
      return cached;
    }

    const source = fs.readFileSync(resolved, "utf8");
    const declarations = new Parser(Lexer.tokenize(source)).parseDeclarations();
    const module = { filePath: resolved, declarations };
    this.modules.set(resolved, module);

    for (const declaration of declarations) {
      if (declaration.kind === "import") {
        this.resolveImport(module, declaration);
      }
    }

    return module;
  }

  private resolveImport(module: SourceModule, declaration: ImportDecl): void {
    const joined = declaration.path.join("/");
    if (joined === "std") {
      const stdDir = this.stdPath ? path.resolve(this.stdPath) : path.resolve("lib/std");
      if (fs.existsSync(stdDir)) {
        for (const entry of fs.readdirSync(stdDir)) {
          if (entry.endsWith(".slx")) {
            this.loadFile(path.join(stdDir, entry));
          }
        }
      }
      return;
    }

    const relativeBase = path.dirname(module.filePath);
    const fileCandidate = path.join(relativeBase, `${declaration.path.at(-1)}.slx`);
    const syxCandidate = path.join(relativeBase, `${declaration.path.at(-1)}.syx`);
    if (fs.existsSync(fileCandidate)) {
      this.loadFile(fileCandidate);
      return;
    }
    if (fs.existsSync(syxCandidate)) {
      this.loadFile(syxCandidate);
      return;
    }
    throw new SlynxError(`Unable to resolve import '${joined}' from '${module.filePath}'`, "ImportError");
  }
}

class SemanticModel {
  readonly modulesByPath = new Map<string, SourceModule>();
  readonly declarations = new Map<string, DeclRef[]>();
  readonly exports = new Map<string, Map<string, DeclRef>>();

  constructor(modules: SourceModule[]) {
    for (const module of modules) {
      this.modulesByPath.set(module.filePath, module);
      const moduleExports = new Map<string, DeclRef>();
      for (const declaration of module.declarations) {
        const ref = declarationToRef(module, declaration);
        if (!ref) continue;
        const bucket = this.declarations.get(nameOfRef(ref)) ?? [];
        bucket.push(ref);
        this.declarations.set(nameOfRef(ref), bucket);
        if (declaration.isPublic) {
          moduleExports.set(nameOfRef(ref), ref);
        }
      }
      this.exports.set(module.filePath, moduleExports);
    }
  }

  moduleScope(module: SourceModule): Map<string, DeclRef> {
    const scope = new Map<string, DeclRef>();

    for (const declaration of module.declarations) {
      const ref = declarationToRef(module, declaration);
      if (ref) {
        scope.set(nameOfRef(ref), ref);
      }
    }

    for (const declaration of module.declarations) {
      if (declaration.kind !== "import") continue;
      const importedExports = this.resolveImportedExports(module, declaration);
      if (declaration.items?.length) {
        for (const item of declaration.items) {
          const ref = importedExports.get(item.name);
          if (!ref) {
            throw new SlynxError(
              `Imported name '${item.name}' is not exported by '${declaration.path.join(".")}'`,
              "ImportError"
            );
          }
          scope.set(item.alias ?? item.name, ref);
        }
        continue;
      }
      for (const [name, ref] of importedExports) {
        scope.set(name, ref);
      }
    }

    return scope;
  }

  resolveImportedExports(module: SourceModule, declaration: ImportDecl): Map<string, DeclRef> {
    const joined = declaration.path.join("/");
    if (joined === "std") {
      const matches = [...this.modulesByPath.values()].filter((candidate) =>
        candidate.filePath.includes(`${path.sep}lib${path.sep}std${path.sep}`)
      );
      if (!matches.length) {
        throw new SlynxError("std import was requested but std modules were not loaded", "ImportError");
      }
      const merged = new Map<string, DeclRef>();
      for (const candidate of matches) {
        for (const [name, ref] of this.exports.get(candidate.filePath) ?? new Map<string, DeclRef>()) {
          merged.set(name, ref);
        }
      }
      return merged;
    }

    const relativeBase = path.dirname(module.filePath);
    for (const extension of [".slx", ".syx"]) {
      const filePath = path.join(relativeBase, `${declaration.path.at(-1)}${extension}`);
      const found = this.modulesByPath.get(path.resolve(filePath));
      if (found) {
        return this.exports.get(found.filePath) ?? new Map<string, DeclRef>();
      }
    }

    throw new SlynxError(`Unable to resolve imported module '${joined}'`, "ImportError");
  }
}

function declarationToRef(module: SourceModule, declaration: Declaration): DeclRef | undefined {
  switch (declaration.kind) {
    case "function":
      return { kind: "function", module, declaration };
    case "object":
      return { kind: "object", module, declaration };
    case "component":
      return { kind: "component", module, declaration };
    case "alias":
      return { kind: "alias", module, declaration };
    case "stylesheet":
      return { kind: "stylesheet", module, declaration };
    default:
      return undefined;
  }
}

function nameOfRef(ref: DeclRef): string {
  return ref.declaration.name;
}

class HirFile {
  constructor(private readonly module: SourceModule) {}

  read(): { declarations: () => Declaration[] } {
    return {
      declarations: () => this.module.declarations.filter((decl) => decl.kind !== "import")
    };
  }
}

export class SlynxHir {
  readonly files: HirFile[] = [];
  readonly modules: SourceModule[] = [];
  readonly symbols = new SemanticModel([]);

  private constructor(modules: SourceModule[]) {
    this.modules = modules;
    this.files = modules.map((module) => new HirFile(module));
    this.symbols = new SemanticModel(modules);
  }

  static fromModules(modules: SourceModule[]): SlynxHir {
    const hir = new SlynxHir(modules);
    hir.validateFunctionCallArity();
    return hir;
  }

  private validateFunctionCallArity(): void {
    for (const module of this.modules) {
      const scope = this.symbols.moduleScope(module);
      for (const declaration of module.declarations) {
        if (declaration.kind === "function") {
          validateArityInBody(declaration.body, scope, this.symbols);
        }
        if (declaration.kind === "object") {
          for (const method of declaration.methods) {
            validateArityInBody(method.body, scope, this.symbols);
          }
        }
      }
    }
  }
}

function validateArityInBody(body: FunctionBody, scope: Map<string, DeclRef>, symbols: SemanticModel): void {
  const visitExpr = (expr: Expr): void => {
    switch (expr.kind) {
      case "call": {
        const signature = resolveCallSignature(expr.callee, scope, symbols);
        if (signature && signature.kind === "function") {
          const expected = signature.declaration.args.length;
          if (expr.args.length !== expected) {
            throw new SlynxError(
              `Invalid function call argument length. Expected ${expected}, received ${expr.args.length}`,
              "InvalidFuncallArgLength"
            );
          }
        }
        visitExpr(expr.callee);
        for (const arg of expr.args) visitExpr(arg);
        return;
      }
      case "namedCall":
        visitExpr(expr.callee);
        for (const arg of expr.args) visitExpr(arg.value);
        return;
      case "component":
        visitExpr(expr.callee);
        for (const entry of expr.entries) visitExpr(entry.expression);
        return;
      case "binary":
        visitExpr(expr.left);
        visitExpr(expr.right);
        return;
      case "field":
        visitExpr(expr.object);
        return;
      case "tupleAccess":
        visitExpr(expr.object);
        return;
      case "tuple":
        for (const member of expr.members) visitExpr(member);
        return;
      case "if":
        visitExpr(expr.condition);
        for (const statement of expr.thenBranch.statements) visitStatement(statement);
        if (expr.thenBranch.tail) visitExpr(expr.thenBranch.tail);
        for (const statement of expr.elseBranch.statements) visitStatement(statement);
        if (expr.elseBranch.tail) visitExpr(expr.elseBranch.tail);
        return;
      case "group":
        visitExpr(expr.expression);
        return;
      default:
        return;
    }
  };

  const visitStatement = (statement: Statement): void => {
    switch (statement.kind) {
      case "let":
        if (statement.value) visitExpr(statement.value);
        return;
      case "assignment":
        visitExpr(statement.target);
        visitExpr(statement.value);
        return;
      case "while":
        visitExpr(statement.condition);
        for (const child of statement.body) visitStatement(child);
        return;
      case "expr":
        visitExpr(statement.expression);
        return;
    }
  };

  if (body.kind === "arrow") {
    visitExpr(body.expression);
    return;
  }
  for (const statement of body.statements) visitStatement(statement);
  if (body.tail) visitExpr(body.tail);
}

function resolveCallSignature(callee: Expr, scope: Map<string, DeclRef>, symbols: SemanticModel): DeclRef | undefined {
  if (callee.kind === "identifier") {
    return scope.get(callee.name);
  }
  if (callee.kind === "field" && callee.object.kind === "identifier") {
    const owner = scope.get(callee.object.name);
    if (owner?.kind === "object") {
      const method = owner.declaration.methods.find((candidate) => candidate.name === callee.field);
      if (method) {
        return { kind: "function", module: owner.module, declaration: method };
      }
    }
  }
  return undefined;
}

export class TypeChecker {
  static check(hir: SlynxHir): SlynxHir {
    for (const module of hir.modules) {
      const scope = hir.symbols.moduleScope(module);
      for (const declaration of module.declarations) {
        switch (declaration.kind) {
          case "function":
            this.checkFunction(module, declaration, scope, undefined, hir.symbols);
            break;
          case "object":
            for (const method of declaration.methods) {
              this.checkFunction(module, method, scope, declaration.name, hir.symbols);
            }
            break;
          case "component":
            this.checkComponent(declaration, scope, hir.symbols);
            break;
          case "stylesheet":
            for (const styleRef of declaration.uses) {
              this.inferExprType(styleRef, { vars: new Map(), objectSelf: undefined }, scope, hir.symbols);
            }
            break;
        }
      }
    }
    return hir;
  }

  private static checkComponent(component: ComponentDecl, scope: Map<string, DeclRef>, symbols: SemanticModel): void {
    for (const item of component.items) {
      if (item.kind !== "prop" || !item.prop.defaultValue || !item.prop.type) continue;
      const actual = this.inferExprType(item.prop.defaultValue, { vars: new Map() }, scope, symbols);
      const expected = normalizeType(item.prop.type, component.name, symbols);
      if (!isAssignable(actual, expected, symbols)) {
        throw new SlynxError(
          `Component prop '${item.prop.name}' expects ${formatType(expected)} but got ${formatType(actual)}`,
          "IncompatibleTypes"
        );
      }
    }
  }

  private static checkFunction(
    module: SourceModule,
    func: FunctionDecl,
    scope: Map<string, DeclRef>,
    objectSelf: string | undefined,
    symbols: SemanticModel
  ): void {
    const env: Env = { vars: new Map(), objectSelf };
    for (const arg of func.args) {
      env.vars.set(arg.name, normalizeType(arg.type, objectSelf, symbols));
    }

    const expectedReturn = normalizeType(func.returnType, objectSelf, symbols);
    if (func.body.kind === "arrow") {
      const actual = this.inferExprType(func.body.expression, env, scope, symbols);
      if (!isAssignable(actual, expectedReturn, symbols)) {
        throw new SlynxError(
          `Function '${func.name}' returns ${formatType(actual)} but expected ${formatType(expectedReturn)}`,
          "IncompatibleTypes"
        );
      }
      return;
    }

    for (const statement of func.body.statements) {
      this.checkStatement(statement, env, scope, symbols);
    }

    if (func.body.tail) {
      const actualTail = this.inferExprType(func.body.tail, env, scope, symbols);
      if (!(expectedReturn.kind === "primitive" && expectedReturn.name === "void")) {
        if (!isAssignable(actualTail, expectedReturn, symbols)) {
          throw new SlynxError(
            `Function '${func.name}' returns ${formatType(actualTail)} but expected ${formatType(expectedReturn)}`,
            "IncompatibleTypes"
          );
        }
      }
      return;
    }

    if (expectedReturn.kind === "primitive" && expectedReturn.name === "void") {
      return;
    }

    throw new SlynxError(
      `Function '${func.name}' is missing a return value of type ${formatType(expectedReturn)}`,
      "MissingReturnValue"
    );
  }

  private static checkStatement(statement: Statement, env: Env, scope: Map<string, DeclRef>, symbols: SemanticModel): void {
    switch (statement.kind) {
      case "let": {
        const inferred = statement.value
          ? this.inferExprType(statement.value, env, scope, symbols)
          : statement.declaredType
            ? normalizeType(statement.declaredType, env.objectSelf, symbols)
            : unknownType();
        if (statement.declaredType) {
          const declared = normalizeType(statement.declaredType, env.objectSelf, symbols);
          if (statement.value && !isAssignable(inferred, declared, symbols)) {
            throw new SlynxError(
              `Variable '${statement.name}' expects ${formatType(declared)} but got ${formatType(inferred)}`,
              "IncompatibleTypes"
            );
          }
          env.vars.set(statement.name, declared);
        } else {
          env.vars.set(statement.name, inferred);
        }
        return;
      }
      case "assignment": {
        const target = this.inferExprType(statement.target, env, scope, symbols);
        const value = this.inferExprType(statement.value, env, scope, symbols);
        if (!isAssignable(value, target, symbols)) {
          throw new SlynxError(
            `Assignment expects ${formatType(target)} but got ${formatType(value)}`,
            "IncompatibleTypes"
          );
        }
        return;
      }
      case "while": {
        const condition = this.inferExprType(statement.condition, env, scope, symbols);
        if (!(condition.kind === "primitive" && condition.name === "bool")) {
          throw new SlynxError("While condition must be bool", "IncompatibleTypes");
        }
        for (const child of statement.body) {
          this.checkStatement(child, env, scope, symbols);
        }
        return;
      }
      case "expr":
        this.inferExprType(statement.expression, env, scope, symbols);
        return;
    }
  }

  private static inferExprType(expr: Expr, env: Env, scope: Map<string, DeclRef>, symbols: SemanticModel): TypeRef {
    switch (expr.kind) {
      case "int":
        return { kind: "primitive", name: "int" };
      case "float":
        return { kind: "primitive", name: "float" };
      case "string":
        return { kind: "primitive", name: "str" };
      case "bool":
        return { kind: "primitive", name: "bool" };
      case "group":
        return this.inferExprType(expr.expression, env, scope, symbols);
      case "identifier": {
        const local = env.vars.get(expr.name);
        if (local) return local;
        if (builtinComponents.has(expr.name)) return { kind: "named", name: expr.name };
        const ref = scope.get(expr.name);
        if (!ref) return unknownType();
        if (ref.kind === "object" || ref.kind === "component" || ref.kind === "stylesheet") {
          return { kind: "named", name: ref.declaration.name };
        }
        if (ref.kind === "alias") {
          return normalizeType(ref.declaration.target, env.objectSelf, symbols);
        }
        return normalizeType(ref.declaration.returnType, env.objectSelf, symbols);
      }
      case "binary": {
        const left = this.inferExprType(expr.left, env, scope, symbols);
        const right = this.inferExprType(expr.right, env, scope, symbols);
        if (["+", "-", "*", "/", "&", "|", "^", "<<", ">>"].includes(expr.op)) {
          if (!isIntLike(left, symbols) || !isIntLike(right, symbols)) {
            throw new SlynxError(`Operator '${expr.op}' requires int operands`, "IncompatibleTypes");
          }
          return { kind: "primitive", name: "int" };
        }
        if (["<", "<=", ">", ">=", "=="].includes(expr.op)) {
          return { kind: "primitive", name: "bool" };
        }
        if (["&&", "||"].includes(expr.op)) {
          if (!isBoolLike(left, symbols) || !isBoolLike(right, symbols)) {
            throw new SlynxError(`Operator '${expr.op}' requires bool operands`, "IncompatibleTypes");
          }
          return { kind: "primitive", name: "bool" };
        }
        return unknownType();
      }
      case "tuple":
        return { kind: "tuple", members: expr.members.map((member) => this.inferExprType(member, env, scope, symbols)) };
      case "tupleAccess": {
        const target = this.inferExprType(expr.object, env, scope, symbols);
        const normalized = fullyResolveType(target, symbols);
        if (normalized.kind !== "tuple") {
          throw new SlynxError("Tuple access target is not a tuple", "InvalidTupleAccessTarget");
        }
        const member = normalized.members[expr.index];
        if (!member) {
          throw new SlynxError(
            `Tuple index ${expr.index} is out of bounds for length ${normalized.members.length}`,
            "InvalidTupleIndex"
          );
        }
        return member;
      }
      case "field": {
        if (expr.object.kind === "identifier") {
          const ownerRef = scope.get(expr.object.name);
          if (ownerRef?.kind === "object") {
            const method = ownerRef.declaration.methods.find((candidate) => candidate.name === expr.field);
            if (method) {
              return normalizeType(method.returnType, ownerRef.declaration.name, symbols);
            }
          }
        }
        const target = fullyResolveType(this.inferExprType(expr.object, env, scope, symbols), symbols);
        if (target.kind !== "named") {
          throw new SlynxError(`Cannot access field '${expr.field}' on ${formatType(target)}`, "IncompatibleTypes");
        }
        const ref = scope.get(target.name) ?? symbols.declarations.get(target.name)?.[0];
        if (ref?.kind === "object") {
          const field = ref.declaration.fields.find((candidate) => candidate.name === expr.field);
          if (field) {
            return normalizeType(field.type, ref.declaration.name, symbols);
          }
          const method = ref.declaration.methods.find((candidate) => candidate.name === expr.field);
          if (method) {
            return normalizeType(method.returnType, ref.declaration.name, symbols);
          }
        }
        if (ref?.kind === "component") {
          const prop = ref.declaration.items.find((item) => item.kind === "prop" && item.prop.name === expr.field);
          if (prop && prop.kind === "prop" && prop.prop.type) {
            return normalizeType(prop.prop.type, ref.declaration.name, symbols);
          }
        }
        return unknownType();
      }
      case "call": {
        const callee = expr.callee;
        if (callee.kind === "field") {
          const ownerType = fullyResolveType(this.inferExprType(callee.object, env, scope, symbols), symbols);
          if (ownerType.kind === "named") {
            const ownerRef = scope.get(ownerType.name) ?? symbols.declarations.get(ownerType.name)?.[0];
            if (ownerRef?.kind === "object") {
              const method = ownerRef.declaration.methods.find((candidate) => candidate.name === callee.field);
              if (method) {
                const args = method.args.filter((arg) => !arg.isSelf);
                if (expr.args.length !== args.length) {
                  throw new SlynxError("Invalid function call argument length", "InvalidFuncallArgLength");
                }
                args.forEach((arg, index) => {
                  const actual = this.inferExprType(expr.args[index], env, scope, symbols);
                  const expected = normalizeType(arg.type, ownerRef.declaration.name, symbols);
                  if (!isAssignable(actual, expected, symbols)) {
                    throw new SlynxError("Method argument type mismatch", "IncompatibleTypes");
                  }
                });
                return normalizeType(method.returnType, ownerRef.declaration.name, symbols);
              }
            }
          }
        }

        const signature = resolveCallSignature(expr.callee, scope, symbols);
        if (signature?.kind === "function") {
          signature.declaration.args.forEach((arg, index) => {
            const actual = this.inferExprType(expr.args[index], env, scope, symbols);
            const expected = normalizeType(arg.type, env.objectSelf, symbols);
            if (!isAssignable(actual, expected, symbols)) {
              throw new SlynxError("Function argument type mismatch", "IncompatibleTypes");
            }
          });
          return normalizeType(signature.declaration.returnType, env.objectSelf, symbols);
        }

        if (expr.callee.kind === "identifier") {
          const ref = scope.get(expr.callee.name) ?? symbols.declarations.get(expr.callee.name)?.[0];
          if (ref?.kind === "stylesheet") {
            return { kind: "named", name: ref.declaration.name };
          }
        }

        return unknownType();
      }
      case "namedCall": {
        if (expr.callee.kind === "identifier") {
          if (expr.callee.name === "Self" && env.objectSelf) {
            return { kind: "named", name: env.objectSelf };
          }
          const ref = scope.get(expr.callee.name) ?? symbols.declarations.get(expr.callee.name)?.[0];
          if (ref?.kind === "object") {
            for (const arg of expr.args) {
              const field = ref.declaration.fields.find((candidate) => candidate.name === arg.name);
              if (!field) continue;
              const actual = this.inferExprType(arg.value, env, scope, symbols);
              const expected = normalizeType(field.type, ref.declaration.name, symbols);
              if (!isAssignable(actual, expected, symbols)) {
                throw new SlynxError("Object field type mismatch", "IncompatibleTypes");
              }
            }
            return { kind: "named", name: ref.declaration.name };
          }
          if (ref?.kind === "stylesheet") {
            return { kind: "named", name: ref.declaration.name };
          }
        }
        if (expr.callee.kind === "field" && expr.callee.object.kind === "identifier" && expr.callee.object.name === "Self") {
          return { kind: "named", name: env.objectSelf ?? "Self" };
        }
        return unknownType();
      }
      case "component": {
        if (expr.callee.kind === "identifier") {
          const ref = scope.get(expr.callee.name) ?? symbols.declarations.get(expr.callee.name)?.[0];
          if (ref?.kind === "component") {
            for (const entry of expr.entries) {
              if (entry.kind !== "prop") continue;
              const prop = ref.declaration.items.find((item) => item.kind === "prop" && item.prop.name === entry.name);
              if (prop && prop.kind === "prop" && prop.prop.type) {
                const actual = this.inferExprType(entry.expression, env, scope, symbols);
                const expected = normalizeType(prop.prop.type, ref.declaration.name, symbols);
                if (!isAssignable(actual, expected, symbols)) {
                  throw new SlynxError("Component prop type mismatch", "IncompatibleTypes");
                }
              }
            }
            return { kind: "named", name: ref.declaration.name };
          }
          if (builtinComponents.has(expr.callee.name)) {
            for (const entry of expr.entries) {
              this.inferExprType(entry.expression, env, scope, symbols);
            }
            return { kind: "primitive", name: "Component" };
          }
        }
        return { kind: "primitive", name: "Component" };
      }
      case "if": {
        const condition = this.inferExprType(expr.condition, env, scope, symbols);
        if (!isBoolLike(condition, symbols)) {
          throw new SlynxError("if condition must be bool", "IncompatibleTypes");
        }
        const thenType = inferBlockTail(expr.thenBranch, env, scope, symbols);
        const elseType = inferBlockTail(expr.elseBranch, env, scope, symbols);
        if (!isAssignable(thenType, elseType, symbols) || !isAssignable(elseType, thenType, symbols)) {
          throw new SlynxError("if branches must return compatible types", "IncompatibleTypes");
        }
        return thenType;
      }
    }
  }
}

function inferBlockTail(block: BlockExpr, env: Env, scope: Map<string, DeclRef>, symbols: SemanticModel): TypeRef {
  const blockEnv: Env = { vars: new Map(env.vars), objectSelf: env.objectSelf };
  for (const statement of block.statements) {
    TypeChecker["checkStatement"](statement, blockEnv, scope, symbols);
  }
  return block.tail ? TypeChecker["inferExprType"](block.tail, blockEnv, scope, symbols) : { kind: "primitive", name: "void" };
}

function normalizeType(type: TypeRef, selfName: string | undefined, symbols: SemanticModel): TypeRef {
  if (type.kind === "self") {
    return selfName ? { kind: "named", name: selfName } : { kind: "named", name: "Self" };
  }
  if (type.kind === "tuple") {
    return { kind: "tuple", members: type.members.map((member) => normalizeType(member, selfName, symbols)) };
  }
  return type;
}

function fullyResolveType(type: TypeRef, symbols: SemanticModel, seen = new Set<string>()): TypeRef {
  if (type.kind === "named") {
    if (seen.has(type.name)) {
      return type;
    }
    const ref = symbols.declarations.get(type.name)?.find((candidate) => candidate.kind === "alias");
    if (ref?.kind === "alias") {
      seen.add(type.name);
      return fullyResolveType(ref.declaration.target, symbols, seen);
    }
  }
  if (type.kind === "tuple") {
    return { kind: "tuple", members: type.members.map((member) => fullyResolveType(member, symbols, seen)) };
  }
  return type;
}

function isAssignable(actual: TypeRef, expected: TypeRef, symbols: SemanticModel): boolean {
  const resolvedActual = fullyResolveType(actual, symbols);
  const resolvedExpected = fullyResolveType(expected, symbols);
  if (resolvedExpected.kind === "primitive" && resolvedExpected.name === "Component") {
    if (resolvedActual.kind === "primitive" && resolvedActual.name === "Component") return true;
    if (resolvedActual.kind === "named") {
      const ref = symbols.declarations.get(resolvedActual.name)?.[0];
      return ref?.kind === "component" || builtinComponents.has(resolvedActual.name);
    }
  }
  if (resolvedActual.kind !== resolvedExpected.kind) return false;
  if (resolvedActual.kind === "primitive" && resolvedExpected.kind === "primitive") {
    return resolvedActual.name === resolvedExpected.name;
  }
  if (resolvedActual.kind === "named" && resolvedExpected.kind === "named") {
    return resolvedActual.name === resolvedExpected.name;
  }
  if (resolvedActual.kind === "tuple" && resolvedExpected.kind === "tuple") {
    return (
      resolvedActual.members.length === resolvedExpected.members.length &&
      resolvedActual.members.every((member, index) => isAssignable(member, resolvedExpected.members[index], symbols))
    );
  }
  return false;
}

function isIntLike(type: TypeRef, symbols: SemanticModel): boolean {
  const resolved = fullyResolveType(type, symbols);
  return resolved.kind === "primitive" && resolved.name === "int";
}

function isBoolLike(type: TypeRef, symbols: SemanticModel): boolean {
  const resolved = fullyResolveType(type, symbols);
  return resolved.kind === "primitive" && resolved.name === "bool";
}

function unknownType(): TypeRef {
  return { kind: "named", name: "unknown" };
}

function formatType(type: TypeRef): string {
  switch (type.kind) {
    case "primitive":
      return type.name;
    case "named":
      return type.name;
    case "self":
      return "Self";
    case "tuple":
      return `(${type.members.map((member) => formatType(member)).join(", ")})`;
  }
}

export class Monomorphizer {
  static resolve(hir: SlynxHir): SlynxHir {
    const aliases = new Map<string, TypeRef>();
    for (const module of hir.modules) {
      for (const declaration of module.declarations) {
        if (declaration.kind === "alias") {
          aliases.set(declaration.name, declaration.target);
        }
      }
    }
    const visiting = new Set<string>();
    const visited = new Set<string>();
    const visit = (name: string): void => {
      if (visited.has(name)) return;
      if (visiting.has(name)) {
        throw new SlynxError(`Recursive type alias '${name}'`, "RecursiveType");
      }
      visiting.add(name);
      const target = aliases.get(name);
      if (target?.kind === "named" && aliases.has(target.name)) {
        visit(target.name);
      }
      visiting.delete(name);
      visited.add(name);
    };
    for (const name of aliases.keys()) {
      visit(name);
    }
    return hir;
  }
}

export class SlynxIR {
  constructor(private readonly lines: string[]) {}

  formatSir(): string {
    return this.lines.join("\n");
  }
}

export class CompilationOutput {
  constructor(private readonly filePath: string, private readonly irValue: SlynxIR) {}

  outputPath(): string {
    return this.filePath;
  }

  ir(): SlynxIR {
    return this.irValue;
  }

  write(): void {
    fs.writeFileSync(this.filePath, this.irValue.formatSir(), "utf8");
  }
}

export class BuildStages {
  constructor(
    private readonly hir: SlynxHir,
    private readonly ir: SlynxIR,
    private readonly sourcePath: string
  ) {}

  hirText(): string {
    const fileNames = this.hir.modules.map((module) => path.basename(module.filePath)).join(", ");
    return `HIR Files: ${fileNames}\n${JSON.stringify(
      this.hir.modules.map((module) => module.declarations.map((decl) => decl.kind)),
      null,
      2
    )}`;
  }

  irText(): string {
    return this.ir.formatSir();
  }

  dumpPath(extension: string): string {
    return replaceExtension(this.sourcePath, extension);
  }

  writeHir(): void {
    fs.writeFileSync(this.dumpPath("hir"), this.hirText(), "utf8");
  }

  writeIr(): void {
    fs.writeFileSync(this.dumpPath("ir"), this.irText(), "utf8");
  }

  intoOutput(): CompilationOutput {
    return new CompilationOutput(this.dumpPath("sir"), this.ir);
  }
}

export class SlynxContext {
  static new(sourcePath: string, stdPath?: string): SlynxContext {
    return new SlynxContext(sourcePath, stdPath);
  }

  constructor(private readonly sourcePath: string, private readonly stdPath?: string) {}

  buildStages(): BuildStages {
    const registry = new ModuleRegistry(this.stdPath);
    const entry = registry.load(this.sourcePath);
    const hir = SlynxHir.fromModules(registry.allModules());
    TypeChecker.check(hir);
    Monomorphizer.resolve(hir);
    const ir = lowerToIr(entry, hir);
    return new BuildStages(hir, ir, path.resolve(this.sourcePath));
  }

  compile(): CompilationOutput {
    return this.buildStages().intoOutput();
  }
}

function lowerToIr(entry: SourceModule, hir: SlynxHir): SlynxIR {
  const lines: string[] = ["module " + path.basename(entry.filePath)];
  for (const module of hir.modules) {
    for (const declaration of module.declarations) {
      switch (declaration.kind) {
        case "function":
          lines.push(`fn ${declaration.name}`);
          collectBodyInstructions(declaration.body, lines);
          break;
        case "object":
          lines.push(`object ${declaration.name}`);
          for (const method of declaration.methods) {
            lines.push(`method ${declaration.name}.${method.name}`);
            collectBodyInstructions(method.body, lines);
          }
          break;
        case "component":
          lines.push(`component ${declaration.name}`);
          break;
        case "stylesheet":
          lines.push(`stylesheet ${declaration.name}`);
          break;
        case "alias":
          lines.push(`alias ${declaration.name}`);
          break;
      }
    }
  }
  return new SlynxIR(lines);
}

function collectBodyInstructions(body: FunctionBody, lines: string[]): void {
  const visitExpr = (expr: Expr): void => {
    switch (expr.kind) {
      case "int":
        lines.push("I32");
        return;
      case "if":
        lines.push("Cbr");
        visitBlock(expr.thenBranch);
        lines.push("Br");
        visitBlock(expr.elseBranch);
        return;
      case "binary":
        visitExpr(expr.left);
        visitExpr(expr.right);
        return;
      case "call":
        visitExpr(expr.callee);
        expr.args.forEach(visitExpr);
        return;
      case "namedCall":
        visitExpr(expr.callee);
        expr.args.forEach((arg) => visitExpr(arg.value));
        return;
      case "component":
        expr.entries.forEach((entry) => visitExpr(entry.expression));
        return;
      case "field":
        visitExpr(expr.object);
        return;
      case "tupleAccess":
        visitExpr(expr.object);
        return;
      case "tuple":
        expr.members.forEach(visitExpr);
        return;
      case "group":
        visitExpr(expr.expression);
        return;
      default:
        return;
    }
  };

  const visitStatement = (statement: Statement): void => {
    switch (statement.kind) {
      case "let":
        if (statement.value) visitExpr(statement.value);
        return;
      case "assignment":
        visitExpr(statement.value);
        return;
      case "while":
        lines.push("Loop");
        visitExpr(statement.condition);
        statement.body.forEach(visitStatement);
        return;
      case "expr":
        visitExpr(statement.expression);
        return;
    }
  };

  const visitBlock = (block: BlockExpr): void => {
    block.statements.forEach(visitStatement);
    if (block.tail) visitExpr(block.tail);
  };

  if (body.kind === "arrow") {
    visitExpr(body.expression);
  } else {
    visitBlock(body);
  }
}

function replaceExtension(filePath: string, extension: string): string {
  const parsed = path.parse(filePath);
  return path.join(parsed.dir, `${parsed.name}.${extension}`);
}

export function compileCode(sourcePath: string, stdPath?: string): void {
  SlynxContext.new(sourcePath, stdPath).compile().write();
}

export function compileToIr(sourcePath: string, stdPath?: string): SlynxIR {
  return SlynxContext.new(sourcePath, stdPath).compile().ir();
}

export function parseDeclarations(source: string): Declaration[] {
  return new Parser(Lexer.tokenize(source)).parseDeclarations();
}

export function buildHirFromSource(source: string): SlynxHir {
  const module: SourceModule = {
    filePath: path.resolve("inline.syx"),
    declarations: parseDeclarations(source)
  };
  return SlynxHir.fromModules([module]);
}

export const lexer = { Lexer };
export const parser = { Parser };
