use std::{
    collections::{
        BTreeSet,
        VecDeque,
    },
    ops::{
        BitAnd,
        BitOr,
    },
    fmt::{
        self,
        Display,
        Formatter,
    },
};

/*
 * TODO
 * ~ `Not` operator
 * ~ `Filter::normalize()` to run over expression tokens before query; e.g. use
 *   this to enforce lowercase searches
 */

pub trait Filter: Sized {
    type FilterSet;
    // Quick performance note on `FilterSet` inner type:
    //
    //      FilterSet<&Data>            - fastest
    //      FilterSet<Rc<Data>>         - 4~5% performance hit
    //      FilterSet<Rc<RefCell<Data>> - ~20% performance hit

    fn collection(&self) -> &Self::FilterSet;

    fn query(&self, expression: &Expression) -> Option<Self::FilterSet>
        where for<'s> &'s Self::FilterSet: BitAnd<Output = Self::FilterSet> + BitOr<Output = Self::FilterSet>,
    {
        Some(query_internal(self, expression.clone(), self.collection())?)
    }

    fn search(&self, needle: &String, superset: &Self::FilterSet) -> Self::FilterSet;
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum Operator {
    And,
    Or,
    Paren,
}

#[derive(Debug, Clone)]
enum Token {
    Item(String),
    Operator(Operator),
}

#[derive(Debug, Clone, Default)]
pub struct Expression {
    stack: VecDeque<Token>,
}

#[derive(Debug)]
pub struct ExpressionError {
    description: &'static str,
}

impl Display for Operator {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Operator::And => "*",
            Operator::Or => "+",
            Operator::Paren => "(",
        })
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Token::Item(item) => write!(f, "{}", item),
            Token::Operator(op) => write!(f, "{}", op),
        }
    }
}

impl Expression {
    fn acquire(&mut self, item: String) -> String {
        self.stack.push_back(Token::Item(item));

        String::new()
    }

    fn error(&self) -> Option<ExpressionError> {
        let mut items = 0;

        for token in &self.stack {
            match token {
                Token::Item(_) => {
                    items += 1;
                }
                Token::Operator(Operator::Paren) => {
                    return Some(ExpressionError::MISSING_CLOSING_PAREN);
                }
                Token::Operator(op) => {
                    if items > 1 {
                        items -= 1;
                    } else {
                        return Some(match op {
                            Operator::Or
                                => ExpressionError::UNEXPECTED_OR,
                            _   => ExpressionError::BUG,
                        })
                    }
                }
            }
        }

        (items > 1).then(|| ExpressionError::BUG)
    }

    fn is_unit(&self) -> bool {
        self.stack.len() == 1 && matches!(self.stack.front(), Some(Token::Item(_)))
    }

    /// A heuristic for whether this expression is simpler than another one (is
    /// it shorter?).
    fn lighter_than(&self, rhs: &Expression) -> bool {
        self.stack.len() < rhs.stack.len()
    }

    fn pop_item(&mut self) -> Option<String> {
        if let Some(Token::Item(item)) = self.stack.pop_back() {
            Some(item)
        } else {
            None
        }
    }

    fn pop_op(&mut self) -> Option<Operator> {
        if let Some(Token::Operator(op)) = self.stack.pop_back() {
            Some(op)
        } else {
            None
        }
    }

    fn push_op(&mut self, op: Operator) {
        self.stack.push_back(Token::Operator(op));
    }

    fn push_op_stack(&mut self, mut op_stack: Vec<Operator>) {
        while let Some(op) = op_stack.pop() {
            self.push_op(op);
        }
    }

    /// Reduce an expression into a left-hand-side, a right-hand-side and an
    /// operator.  Check if the expression `is_unit()` before doing this.
    fn reduce(mut self) -> Option<(Expression, Expression, Operator)> {
        let mut rhs = Self::default();
        let mut balance = 0;
        let op = self.pop_op()?;

        while let Some(token) = self.stack.pop_back() {
            match token {
                Token::Item(_) => balance += 1,
                Token::Operator(_) => balance -= 1,
            }

            rhs.stack.push_front(token);

            if balance == 1 {
                // Completed right-hand expression
                break;
            }
        }

        if balance == 1 {
            Some((self, rhs, op))
        } else {
            None
        }
    }

    fn into_result(self) -> Result<Self, ExpressionError> {
        if let Some(error) = self.error() {
            Err(error)
        } else {
            Ok(self)
        }
    }
}

impl Display for Expression {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self
            .stack
            .iter()
            .map(|token| token.to_string())
            .collect::<String>()
        )
    }
}

impl TryFrom<String> for Expression {
    type Error = ExpressionError;

    fn try_from(string: String) -> Result<Expression, ExpressionError> {
        Expression::try_from(string.as_str())
    }
}

impl TryFrom<&String> for Expression {
    type Error = ExpressionError;

    fn try_from(string: &String) -> Result<Expression, ExpressionError> {
        Expression::try_from(string.as_str())
    }
}

impl TryFrom<&str> for Expression {
    type Error = ExpressionError;

    /// Attempt to convert an infix string expression to a postfix `Expression`.
    /// Expression syntax includes parentheses, ' ' for AND and `|` for OR.
    ///
    /// E.g.:
    ///         A B             = A AND B
    ///         A B (C | D)     = A AND B AND (C OR D)
    ///
    /// Forbidden syntax includes "()", "(   )", etc.
    fn try_from(string: &str) -> Result<Expression, ExpressionError> {
        let mut expr = Expression::default();
        let mut operators = vec!();
        let mut item = String::new();
        let mut implicit_and = false;

        // `implicit_and` also implies the last non-space character visited was
        // not syntactic (i.e. an item).

        for character in string.chars() {
            match character {
                '|' | '(' | ')' | ' ' | '\t' => {
                    if !item.is_empty() {
                        item = expr.acquire(item);
                    }
                }
                _ => {}
            }

            match character {
                '|' | ')' | ' ' | '\t' => {}
                _ => {
                    if implicit_and && item.is_empty() {
                        loop {
                            match operators.pop() {
                                Some(op @ Operator::Paren) |
                                Some(op @ Operator::Or)
                                    => { operators.push(op); break; }
                                Some(op @ Operator::And)
                                    => { expr.push_op(op); }
                                _   => { break; }
                            }
                        }
                        
                        operators.push(Operator::And);
                    }
                }
            }

            match character {
                '|' => {
                    if !implicit_and {
                        return Err(ExpressionError::UNEXPECTED_OR);
                    }

                    loop {
                        match operators.pop() {
                            Some(op @ Operator::Paren)
                                => { operators.push(op); break; }
                            Some(op @ Operator::And) |
                            Some(op @ Operator::Or)
                                => { expr.push_op(op); }
                            _   => { break; }
                        }
                    }

                    operators.push(Operator::Or);
                    implicit_and = false;
                }
                '(' => {
                    operators.push(Operator::Paren);
                    implicit_and = false;
                }
                ')' => {
                    if !implicit_and {
                        return Err(ExpressionError::UNEXPECTED_CLOSING_PAREN);
                    }

                    loop {
                        match operators.pop() {
                            Some(Operator::Paren)
                                => { break; }
                            Some(op @ Operator::And) |
                            Some(op @ Operator::Or)
                                => { expr.push_op(op); }
                            None
                                => { return Err(ExpressionError::MISSING_OPENING_PAREN); }
                        }
                    }
                }
                ' ' | '\t' => {}
                _ => {
                    item.push(character);
                    implicit_and = true;
                }
            }
        }

        if item.len() > 0 {
            expr.acquire(item);
        }

        expr.push_op_stack(operators);
        expr.into_result()
   }
}

impl ExpressionError {
    const BUG: Self = Self::new("Parsing error (implementation bug)");
    const UNEXPECTED_CLOSING_PAREN: Self = Self::new("Unexpected token ')'");
    const MISSING_OPENING_PAREN: Self = Self::new("Missing opening parenthesis");
    const MISSING_CLOSING_PAREN: Self = Self::new("Missing closing parenthesis");
    const UNEXPECTED_OR: Self = Self::new("Unexpected token '|'");

    const fn new(description: &'static str) -> Self {
        Self {
            description: description,
        }
    }
}

fn query_internal<S>(filter: &impl Filter<FilterSet = S>, mut expression: Expression, superset: &S) -> Option<S>
    where for<'s> &'s S: BitAnd<Output = S> + BitOr<Output = S>,
{
    if expression.is_unit() {
        Some(filter.search(&expression.pop_item()?, superset))
    } else {
        let (mut lhs, mut rhs, op) = expression.reduce()?;

        match op {
            Operator::And => {
                if lhs.lighter_than(&rhs) {
                    (lhs, rhs) = (rhs, lhs);
                }

                query_internal(filter, lhs, &query_internal(filter, rhs, superset)?)
            }
            Operator::Or => {
                Some(&query_internal(filter, lhs, superset)? | &query_internal(filter, rhs, superset)?)
            }
            _ => None
        }
    }
}

