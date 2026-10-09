---
version: 0.1
status: draft
---

# Chalk Language Specification

## 1. Scope

This specification defines an equation-based language with:

- A persistent global workspace.
- Anonymous and named models.
- Model-based values.
- Symbolic unknowns.
- Value constraints.
- Lazy queries.

## 2. Terminology

A symbol is an unknown model with a stable identity and domain.

A constraint is a condition that restricts admissible assignments.

A model is an immutable collection of symbols, constraints,
nested models, and observations.

A workspace is a persistent constraint context.

A solution is an assignment satisfying all domain restrictions and
constraints in a context.

A residual model describes unresolved relationships.

## 3. Semantic Principles

1. "=" adds an equality constraint; it does not overwrite a previous value.
2. Equations in a model are simultaneous.
3. Cyclic dependencies are permitted.
4. Models are immutable.
5. Distinct instances have distinct local symbol identities.
6. Solver failure is distinct from inconsistency.
7. Multiple solutions must not be reduced to an arbitrary solution.
8. Solver capabilities do not determine program meaning.
9. Undefined arithmetic must not be treated as a satisfying equality.

## 4. Lexical Syntax

### 4.1 Character Set

Source files are UTF-8.

Identifiers and language tokens use ASCII characters in version 0.1.

### 4.2 Identifiers

identifier = (letter | "_"), { letter | digit | "_" } ;
letter     = "A" ... "Z" | "a" ... "z" ;
digit      = "0" ... "9" ;

Identifiers are case-sensitive.

### 4.3 Numeric Literals

integer  = digit, { digit } ;
exponent = ("e" | "E"), [ "+" | "-" ], integer ;
float    = integer, ".", integer, [ exponent ]
         | integer, exponent ;

Examples:

    0
    123
    0.5
    12.0
    1e3
    1.25e-3

Signs are separate tokens.

The forms ".5" and "5." are invalid.

Integer literal magnitudes must not exceed 2^63.

The magnitude 2^63 is valid only as the immediate operand of unary
minus, producing the minimum i64 value.

Floating-point literals are rounded to binary64 using
round-to-nearest, ties-to-even.

A literal that rounds to a nonfinite value is invalid.

### 4.4 Whitespace and Comments

Spaces and tabs are ignored.

A comment starts with "//" and continues to the end of the line.

CRLF and LF are recognized as newlines.

A newline is a statement separator unless it occurs inside parentheses.

Newlines inside model braces remain statement separators.

### 4.5 Token Recognition

The lexer uses longest-match token recognition.

No implicit multiplication is inserted between adjacent tokens.

## 5. Concrete Syntax

EBNF conventions:

- Quoted text denotes a terminal.
- { X } denotes zero or more repetitions.
- [ X ] denotes an optional occurrence.
- "|" denotes alternatives.
- "," denotes concatenation.

### 5.1 Grammar

program =
    statement_list ;

statement_list =
    { separator },
    [ statement,
      { separators, statement },
      { separator } ] ;

separator =
    ";" | NEWLINE ;

separators =
    separator, { separator } ;

statement =
    domain_declaration
  | named_model
  | equation
  | assertion
  | query ;

domain_declaration =
    identifier, ":", numeric_type ;

numeric_type =
    "i64" | "f64" ;

named_model =
    identifier, model ;

model =
    "{", statement_list, "}" ;

equation =
    expression, "=", expression ;

assertion =
    expression, "==", expression ;

query =
    expression ;

expression =
    sum ;

sum =
    product, { ("+" | "-"), product } ;

product =
    unary, { ("*" | "/"), unary } ;

unary =
    ("+" | "-"), unary
  | power ;

power =
    postfix, [ "^", unary ] ;

postfix =
    primary, { suffix } ;

suffix =
    ".", identifier
  | "(", [ arguments ], ")" ;

arguments =
    argument, { ",", argument } ;

argument =
    identifier, "=", expression ;

primary =
    integer
  | float
  | identifier
  | "(", expression, ")"
  | model ;

### 5.2 Precedence

Highest to lowest:

1. Member selection and specialization.
2. Exponentiation.
3. Unary signs.
4. Multiplication and division.
5. Addition and subtraction.
6. Statement-level "=" and "==".

Addition, subtraction, multiplication, and division associate left.

Exponentiation associates right.

Examples:

    a - b - c = (a - b) - c
    a ^ b ^ c = a ^ (b ^ c)
    -2 ^ 2 = -(2 ^ 2)
    2 ^ -3 = 2 ^ (-3)

These examples describe parsing, not source equations to execute.

### 5.3 Structural Restrictions

The following forms are invalid:

    2x
    x = y = 3
    f(3)
    f(t) = t*t
    [x, y]

A named model declaration does not use "=":

    motion {
        distance = speed * time
    }

## 6. Abstract Syntax

Program:
    Program(List<Statement>)

Statement:
    DomainDeclaration(Name, NumericType)
    NamedModel(Name, ModelBody)
    Equation(Expression, Expression)
    AssertEqual(Expression, Expression)
    Query(Expression)

Expression:
    IntegerLiteral(Magnitude)
    FloatLiteral(Binary64)
    NameReference(Name)
    Unary(UnaryOperator, Expression)
    Binary(BinaryOperator, Expression, Expression)
    Member(Expression, Name)
    Specialize(Expression, List<Argument>)
    ModelLiteral(ModelBody)

Argument:
    NamedArgument(Name, Expression)

UnaryOperator:
    Positive
    Negative

BinaryOperator:
    Add
    Subtract
    Multiply
    Divide
    Power

NumericType:
    I64
    F64

ModelBody:
    List<Statement>

Parentheses, comments, and statement separators are not AST nodes.

Source spans are retained as metadata.

## 7. Semantic Representation

### 7.1 Identities

SymbolId, ModelId, InstanceId, and ConstraintId are opaque identities.

Textual names are not identities.

### 7.2 Symbols

A symbol is:

    s = (id, name, owner, domain)

where:

    id     : SymbolId
    name   : Identifier
    owner  : InstanceId
    domain : I64 | F64

### 7.3 Templates

A model template is:

    M = (S, D, C, N, O)

where:

    S = locally declared scalar symbols
    D = scalar domain declarations
    C = equality constraints
    N = named nested templates
    O = observations

### 7.4 Instances

An instance is:

    I = (id, S', D', C', N', O')

Instantiation creates an injective renaming:

    alpha : S -> S'

Every reference to a local symbol is renamed consistently.

Nested templates receive fresh default instances.

### 7.5 Workspace

A workspace contains:

    W = (symbols, domains, constraints, templates, default_instances)

The workspace persists between top-level submissions.

## 8. Name Resolution and Scope

### 8.1 Scalar Names

An unqualified scalar name denotes a symbol owned by the current context.

Repeated occurrences in that context denote the same symbol.

An undeclared scalar name introduces a symbol.

Within a model body, scalar names are collected from:

- Domain declarations.
- Equation operands.
- Query expressions.
- Assertion operands.

Argument labels and member labels do not introduce caller symbols.

### 8.2 Domain Selection

Explicit domain declarations determine symbol domains.

Otherwise, a symbol defaults to F64.

Example:

    count: i64
    count = 3

A model body is resolved as a whole. Declaration order does not affect
domain selection.

In the global workspace, a symbol's domain is fixed when introduced.

A later declaration may repeat the same domain but may not change it.

### 8.3 Scalar Isolation

A model does not implicitly capture scalar symbols from an outer context.

Example:

    y = 100

    m {
        x = y + 1
    }

The two occurrences of "y" belong to different contexts.

Connections require qualified references:

    m.y = y

### 8.4 Model Names

Named models are lexically visible in their declaring scope and child scopes.

Each scope is checked for model/scalar name collisions.

A name may not denote both a local scalar and a local model.

Duplicate model names in one scope are invalid.

Recursive model construction is invalid in version 0.1.

### 8.5 Members

Member selection requires a model instance or template reference.

A member must exist in the selected model.

Member selection does not create missing members.

When a member is selected directly from a named template reference,
the current context's default instance is used.

## 9. Static Semantics

Expression categories:

    Scalar(I64)
    Scalar(F64)
    Template
    Instance
    ScalarView

A ScalarView identifies a scalar together with its owning context.

Arithmetic operands must be scalar-valued.

Equation and assertion operands must be scalar-valued.

Member selection requires a model-valued base.

Specialization requires:

- A model template reference.
- A model instance.
- A scalar view.

Named argument values must be scalar expressions.

An argument label must identify a scalar member of the target context.

Duplicate argument labels are invalid.

Model-valued equality is invalid.

## 10. Numeric Semantics

### 10.1 Domains

I64 consists of signed integers in:

    [-2^63, 2^63 - 1]

F64 consists of finite IEEE 754 binary64 values.

Positive and negative zero are permitted and compare numerically equal.

NaN and infinities are excluded from admissible scalar values.

### 10.2 Integer Operations

Integer addition, subtraction, multiplication, and negation are checked.

An overflowing operation is undefined.

Integer exponentiation is used when both operands are I64 and the
exponent is nonnegative.

Integer exponentiation is checked.

For this operation:

    0^0 = 1

### 10.3 Mixed Operations

When an operation combines I64 and F64 operands, the integer operand is
converted to binary64 using round-to-nearest, ties-to-even.

The operation then uses floating-point semantics.

### 10.4 Division

Division always produces F64.

Integer operands are converted before division.

Division by either positive or negative zero is undefined.

### 10.5 Floating-Point Operations

Floating-point addition, subtraction, multiplication, and division use
binary64 arithmetic with round-to-nearest, ties-to-even.

Operations must not be reassociated or fused implicitly.

A nonfinite result is undefined.

Floating-point exponentiation uses the version 0.1 reference pow
implementation.

An implementation must identify and pin that implementation.

Replacing it with another pow implementation is not assumed to preserve
all results.

### 10.6 Equality

Two I64 values compare as integers.

If either operand is F64, both operands compare as F64 after conversion.

Equality is exact under these numeric rules.

No tolerance is applied implicitly.

## 11. Expression Evaluation

An assignment rho maps symbol identities to domain-admissible values.

Expression evaluation is a partial function:

    Eval(e, rho) = value
    Eval(e, rho) = Undefined

Basic rules:

    Eval(IntegerLiteral(n), rho) = n
    Eval(FloatLiteral(v), rho) = v
    Eval(SymbolReference(s), rho) = rho(s)

For a binary operator op:

    Eval(Binary(op, a, b), rho)
        = Apply(op, Eval(a, rho), Eval(b, rho))

If either operand or the operation is undefined, the result is undefined.

Evaluation follows the AST operation order.

## 12. Constraint Semantics

An assignment satisfies an equality constraint iff both sides are
defined and numerically equal:

    rho |= (a = b)

iff:

    Eval(a, rho) != Undefined
    Eval(b, rho) != Undefined
    Equal(Eval(a, rho), Eval(b, rho))

For a context C:

    Sol(C) =
        { rho |
          rho respects all symbol domains
          and rho satisfies every constraint in C }

A context is inconsistent iff:

    Sol(C) = {}

A context is underdetermined when multiple assignments remain possible.

A query may be uniquely determined even when its context is underdetermined.

## 13. Equation Execution

Executing an equation adds it to the current constraint store:

    C --(a = b)--> C union { a = b }

Consequences:

- Equations do not overwrite.
- Repeating a constraint does not change the solution set.
- Cycles are valid.
- Contradictory constraints are retained.
- Inconsistency does not imply a syntax error.

Example:

    x = 3
    x = 4

The resulting context is inconsistent.

Constraint removal and workspace reset are external workspace operations,
not part of version 0.1 source syntax.

## 14. Model Construction

### 14.1 Anonymous Models

A model literal constructs an immutable template.

Its equations are not added to the enclosing context.

Example:

    {
        x = y + 1
    }

### 14.2 Named Models

A named model declaration registers an immutable template.

Example:

    offset {
        x = y + 1
    }

The declaring context also receives a default instance of the template.

A declaration does not assert scalar equality involving the model name.

### 14.3 Default Instances

Direct member references use default instances:

    offset.y = 3
    offset.x

Constraints added to the default instance do not modify the template.

A subsequent call to the named template does not inherit those constraints.

## 15. Specialization

### 15.1 Template Specialization

For a template M and arguments A:

    Specialize(M, A)
        = FreshInstance(M) with ArgumentConstraints(A)

The returned instance is independent of all previous instances.

### 15.2 Instance Specialization

Specializing an existing instance creates a fresh copy of its local
symbols and accumulated constraints.

It does not mutate the source instance.

Explicit external dependencies are preserved.

### 15.3 Arguments

An argument:

    name = expression

adds a local constraint between the target member and the argument expression.

Argument expressions resolve in the caller's context.

Argument labels resolve in the target context.

Arguments are simultaneous.

Argument order does not affect meaning.

### 15.4 Symbol Specialization

For a scalar view:

    f(t=3)

specialization copies the connected constraint component containing f,
adds the argument constraints, and returns a view of the copied f.

The component includes all constraints transitively connected through
shared symbol identities.

Argument names must identify unambiguous symbols in that component.

### 15.5 Member Specialization

For a member view:

    model.output(args)

specialization uses the member's owning instance and returns the
corresponding output view in the specialized copy.

For an unmodified default instance:

    model.output(args)

is equivalent to:

    model(args).output

If the default instance has additional constraints, the former includes
them and the latter specializes the immutable template.

### 15.6 Locality

Argument constraints do not persist in the caller's workspace.

## 16. Query Semantics

For scalar expression e in context C:

    Values(C, e) =
        { Eval(e, rho) |
          rho in Sol(C)
          and Eval(e, rho) != Undefined }

Also define:

    UndefinedPossible(C, e)

iff:

    exists rho in Sol(C):
        Eval(e, rho) = Undefined

A determined query requires:

    Sol(C) != {}
    UndefinedPossible(C, e) = false
    Values(C, e) = { v }

Its result is v.

A query of a model returns a model value or residual model description.

## 17. Query Outcomes

Semantic answer forms:

    Determined(value)
    Residual(expression_or_model)
    Alternatives(branches)
    Inconsistent
    PossiblyUndefined(details)

Implementation status:

    Complete
    Partial
    Unknown(reason)

An implementation may return a residual representation without enumerating
the entire solution set.

Branches must preserve correlations between queried quantities.

Unknown must not be reported as Inconsistent.

A single satisfying assignment must not be reported as a unique solution
without justification.

Numerical approximations must be marked as approximate.

## 18. Assertion Semantics

An assertion:

    a == b

does not add a constraint.

For a nonempty solution set, if both operands are defined in every solution:

    Proven:
        equality holds in every solution

    Refuted:
        equality holds in no solution

    Undetermined:
        equality holds in some solutions and fails in others

Additional outcomes:

    InconsistentContext
    PossiblyUndefined
    Unknown

Only Proven is a successful assertion.

An inconsistent context does not cause all assertions to pass vacuously.

An assertion failure does not modify the constraint store.

## 19. Observations and Execution Order

### 19.1 Model Bodies

Model construction:

1. Collect declarations.
2. Resolve names and domains.
3. Collect equations.
4. Construct the immutable template.
5. Store queries and assertions as observations.

Observations are not executed during template construction.

Instantiation alone does not execute stored observations.

A host may explicitly request evaluation of an instance's observations.

### 19.2 Global Execution

Top-level statements execute in source order.

Queries and assertions observe the workspace state at their execution point.

Example:

    x = y + 1
    x
    y = 4
    x

The first query may be unresolved.

The second query is determined if supported by the implementation.

Reordering equations preserves the final constraint conjunction but may
change intermediate observations.

## 20. Model Values and First-Class Behavior

Anonymous templates and specialized instances are model-valued expressions.

They support:

- Construction.
- Specialization.
- Member selection.
- Querying.
- Representation as results.

Version 0.1 does not define:

- Model-valued unknowns.
- Model equality.
- General model-result bindings.
- Higher-order model parameters.
- Ordinary function values with positional application.

These require an explicit extension.

A scalar relation must not automatically be treated as a total,
single-valued function.

## 21. Errors

Static errors include:

    InvalidToken
    InvalidLiteral
    UnexpectedToken
    DuplicateModelName
    ModelScalarNameCollision
    ConflictingDomainDeclaration
    MissingMember
    InvalidOperandCategory
    InvalidSpecializationTarget
    UnknownArgumentName
    AmbiguousArgumentName
    DuplicateArgumentName
    RecursiveModelConstruction

Arithmetic undefinedness is part of evaluation semantics.

Inconsistency is a property of a constraint context, not a static error.

Solver timeout or unsupported reasoning is an implementation status,
not proof of inconsistency.

## 22. Solver Contract

A conforming query engine must:

1. Preserve symbol identity and scope.
2. Respect declared numeric domains.
3. Respect overflow and definedness rules.
4. Preserve solution correlations.
5. Distinguish inconsistency from inability to solve.
6. Distinguish one example solution from a unique solution.
7. Preserve source constraints during local specialization.
8. Label approximate results.
9. Avoid transformations valid only over real arithmetic when operating
   under machine-number semantics.
10. Apply explicit resource limits without changing program meaning.

A backend using real arithmetic is not automatically a sound backend for
F64 constraints.

A backend using unbounded integers is not automatically a sound backend
for I64 constraints.

## 23. Required Implementation Separation

Recommended pipeline:

    Source
      -> Tokens with spans
      -> Syntax AST
      -> Resolved symbols and models
      -> Instantiated constraint context
      -> Query
      -> Reasoning backend
      -> Structured outcome

Parser logic must not depend on solver success.

Model representation must not depend on one backend's internal AST.

## 24. Conformance Examples

### 24.1 Partial Specialization

    f = t*t + p
    f(t=3)

Expected:

    A residual answer equivalent to f = 9 + p,
    subject to the specified numeric semantics.

### 24.2 Complete Specialization

    f = t*t + p
    f(t=3, p=1)

Expected:

    Determined(10.0)

The undeclared symbols have domain F64.

### 24.3 Named Model

    offset {
        x = y + 1
    }

    offset(y=3).x

Expected:

    Determined(4.0)

### 24.4 Independent Calls

    offset(y=3).x
    offset(y=8).x

Expected:

    Determined(4.0)
    Determined(9.0)

The calls do not constrain each other.

### 24.5 Inconsistency

    x = 3
    x = 4
    x

Expected:

    Inconsistent

### 24.6 Cyclic Constraints

    {
        x = y - 1
        y = x^2
    }

Expected:

    A valid model.

The implementation may return a residual model or a supported solver result.

Real-arithmetic roots are not automatically exact F64 solutions.

### 24.7 Local Symbol Identity

    a {
        x = 1
    }

    b {
        x = 2
    }

Expected:

    a.x and b.x are distinct symbols.

### 24.8 Assertion Does Not Constrain

    x == 3
    x

Expected:

    The assertion does not add x = 3.
    The query remains unresolved unless other constraints determine x.
