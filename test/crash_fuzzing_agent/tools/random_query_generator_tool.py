"""
Random Query Generator tool for generating random SQL queries using SQLGlot.
"""

import random
import sqlglot
from sqlglot import select, table, column, func, and_, or_, case
from sqlglot import expressions as exp
class RandomQueryGenerator:
    """Random SQL query generator using SQLGlot with Snowflake syntax support."""

    def __init__(self, database: str = "embucket", schema: str = "public"):
        # Table schemas that match the actual database setup with type information
        self.table_schemas = {
            "users": {
                "id": "integer", "user_id": "integer", "name": "string", "email": "string",
                "age": "integer", "created_at": "timestamp", "updated_at": "timestamp"
            },
            "orders": {
                "id": "integer", "order_id": "integer", "user_id": "integer", "amount": "decimal",
                "total": "decimal", "status": "string", "order_date": "timestamp", "created_at": "timestamp"
            },
            "products": {
                "id": "integer", "product_id": "integer", "name": "string", "description": "string",
                "price": "decimal", "quantity": "integer", "created_at": "timestamp"
            },
            "customers": {
                "id": "integer", "customer_id": "integer", "name": "string", "email": "string",
                "age": "integer", "created_at": "timestamp"
            },
            "sales": {
                "id": "integer", "customer_id": "integer", "product_id": "integer", "amount": "decimal",
                "quantity": "integer", "created_at": "timestamp"
            },
            "inventory": {
                "id": "integer", "product_id": "integer", "quantity": "integer",
                "count": "integer", "updated_at": "timestamp"
            },
            "employees": {
                "id": "integer", "user_id": "integer", "name": "string", "email": "string",
                "age": "integer", "created_at": "timestamp"
            },
            "departments": {
                "id": "integer", "name": "string", "description": "string", "created_at": "timestamp"
            },
            "categories": {
                "id": "integer", "name": "string", "description": "string", "created_at": "timestamp"
            },
            "suppliers": {
                "id": "integer", "name": "string", "email": "string", "created_at": "timestamp"
            },
            "transactions": {
                "id": "integer", "user_id": "integer", "amount": "decimal", "type": "string",
                "status": "string", "created_at": "timestamp"
            },
            "accounts": {
                "id": "integer", "user_id": "integer", "name": "string", "type": "string", "created_at": "timestamp"
            },
            "payments": {
                "id": "integer", "order_id": "integer", "amount": "decimal", "status": "string", "created_at": "timestamp"
            },
            "reviews": {
                "id": "integer", "product_id": "integer", "user_id": "integer", "title": "string",
                "description": "string", "created_at": "timestamp"
            },
            "sessions": {
                "id": "integer", "user_id": "integer", "status": "string", "created_at": "timestamp", "updated_at": "timestamp"
            },
            "logs": {
                "id": "integer", "user_id": "integer", "type": "string", "description": "string", "created_at": "timestamp"
            }
        }

        # Common table names for testing
        self.table_names = list(self.table_schemas.keys())

        # Type-aware function mappings
        self.type_compatible_functions = {
            "string": ["UPPER", "LOWER", "LENGTH", "SUBSTR", "CONCAT"],
            "integer": ["ABS", "ROUND"],
            "decimal": ["ABS", "ROUND"],
            "timestamp": ["DATEADD", "DATEDIFF"],
            "any": ["COALESCE", "CURRENT_TIMESTAMP", "CURRENT_DATE", "RANDOM"]
        }

        # Store database and schema for fully qualified table names
        self.database = database
        self.schema = schema

        # Success rate control - what percentage of queries should be "safe"
        self.safe_query_probability = 0.24  # Iteration 2: More edge cases  # Iteration 1: More aggressive  # Iteration 1: More edge cases  # Iteration 3: More edge cases  # Iteration 2: More edge cases  # Iteration 1: More edge cases  # Iteration 1: More aggressive  # 40% of queries will be safer/more likely to succeed

    def _random_table(self):
        """Generate a random table reference with simple table name."""
        table_name = random.choice(self.table_names)
        return table(table_name)

    def _random_column(self, table_alias=None, table_name=None, column_type=None):
        """Generate a random column reference that exists in the specified table."""
        if table_name and table_name in self.table_schemas:
            # Use columns from the specific table
            available_columns = list(self.table_schemas[table_name].keys())
            if column_type:
                # Filter by type if specified
                available_columns = [col for col in available_columns
                                   if self.table_schemas[table_name][col] == column_type]
                if not available_columns:
                    available_columns = list(self.table_schemas[table_name].keys())
        elif table_alias and hasattr(self, '_current_table_schemas'):
            # Use columns from the table associated with the alias
            available_columns = self._current_table_schemas.get(table_alias, ["id", "name", "created_at"])
        else:
            # Fallback to common columns that exist in most tables
            available_columns = ["id", "name", "created_at"]

        col_name = random.choice(available_columns)
        if table_alias:
            return column(col_name, table=table_alias)
        return column(col_name)

    def _get_column_type(self, col_name, table_name):
        """Get the type of a column from a table."""
        if table_name in self.table_schemas and col_name in self.table_schemas[table_name]:
            return self.table_schemas[table_name][col_name]
        return "string"  # Default fallback

    def _random_literal(self, target_type=None):
        """Generate a random literal value, optionally matching a target type."""
        if target_type:
            if target_type == "string":
                strings = ["test", "example", "data", "value", "item", "record"]
                return exp.Literal.string(random.choice(strings))
            elif target_type in ["integer", "decimal"]:
                return exp.Literal.number(random.randint(1, 1000))
            elif target_type == "timestamp":
                # For timestamp comparisons, use string literals that can be cast
                dates = ["2024-01-01", "2023-12-31", "2024-06-15"]
                return exp.Literal.string(random.choice(dates))
            else:
                return exp.Literal.string("test")

        # Random type selection (for fuzzing)
        if random.random() < self.safe_query_probability:
            # Much safer literals - only strings and numbers
            literal_type = random.choice(["string", "number"])
        else:
            # Fuzzier literals - include booleans and nulls but less frequently
            literal_type = random.choice(["string", "number", "string", "number", "boolean", "null"])

        if literal_type == "string":
            strings = ["test", "example", "data", "value", "item", "record"]
            return exp.Literal.string(random.choice(strings))
        elif literal_type == "number":
            return exp.Literal.number(random.randint(1, 1000))
        elif literal_type == "boolean":
            return exp.Boolean(this=random.choice([True, False]))
        else:
            return exp.Null()

    def _random_condition(self, table_alias=None, table_name=None):
        """Generate a random WHERE condition with better type awareness."""
        condition_type = random.choice(["comparison", "in", "like", "between", "is_null"])

        if condition_type == "comparison":
            col = self._random_column(table_alias, table_name)
            col_name = col.name if hasattr(col, 'name') else str(col).split('.')[-1]
            col_type = self._get_column_type(col_name, table_name)

            op = random.choice(["=", "!=", ">", "<", ">=", "<="])

            # Generate type-compatible value
            if random.random() < self.safe_query_probability:
                value = self._random_literal(target_type=col_type)
            else:
                value = self._random_literal()  # Fuzzy - might be incompatible

            if op == "=":
                return exp.EQ(this=col, expression=value)
            elif op == "!=":
                return exp.NEQ(this=col, expression=value)
            elif op == ">":
                return exp.GT(this=col, expression=value)
            elif op == "<":
                return exp.LT(this=col, expression=value)
            elif op == ">=":
                return exp.GTE(this=col, expression=value)
            else:  # "<="
                return exp.LTE(this=col, expression=value)

        elif condition_type == "in":
            col = self._random_column(table_alias, table_name)
            col_name = col.name if hasattr(col, 'name') else str(col).split('.')[-1]
            col_type = self._get_column_type(col_name, table_name)

            if random.random() < self.safe_query_probability:
                # Generate type-consistent values - all same type
                target_type = col_type if col_type in ["string", "integer", "decimal"] else "string"
                values = [self._random_literal(target_type=target_type) for _ in range(random.randint(2, 3))]
            else:
                # Mix types for fuzzing
                values = [self._random_literal() for _ in range(random.randint(2, 5))]
            return col.isin(*values)

        elif condition_type == "like":
            # LIKE only works with string columns
            if random.random() < self.safe_query_probability:
                col = self._random_column(table_alias, table_name, column_type="string")
            else:
                col = self._random_column(table_alias, table_name)  # Might be wrong type
            pattern = random.choice(["%test%", "data%", "%value", "item_"])
            return col.like(exp.Literal.string(pattern))

        elif condition_type == "between":
            col = self._random_column(table_alias, table_name)
            col_name = col.name if hasattr(col, 'name') else str(col).split('.')[-1]
            col_type = self._get_column_type(col_name, table_name)

            if col_type in ["integer", "decimal"] and random.random() < self.safe_query_probability:
                # Use numeric range for numeric columns
                low = exp.Literal.number(random.randint(1, 50))
                high = exp.Literal.number(random.randint(51, 100))
            else:
                # Fuzzy - might use wrong types
                low = self._random_literal()
                high = self._random_literal()
            return col.between(low, high)

        else:  # is_null
            col = self._random_column(table_alias, table_name)
            return col.is_(exp.Null()) if random.choice([True, False]) else col.is_(exp.Null()).not_()

    def _random_function(self, table_name=None):
        """Generate a type-aware random function call."""
        if random.random() < self.safe_query_probability:
            # Safe mode - use type-compatible functions
            return self._safe_random_function(table_name)
        else:
            # Fuzzy mode - might use incompatible types
            return self._fuzzy_random_function(table_name)

    def _safe_random_function(self, table_name=None):
        """Generate a type-safe function call."""
        # Choose function category
        func_category = random.choice(["string", "numeric", "timestamp", "any"])

        if func_category == "string":
            func_name = random.choice(["UPPER", "LOWER", "LENGTH", "SUBSTR", "CONCAT"])
            if func_name in ["UPPER", "LOWER", "LENGTH"]:
                col = self._random_column(table_name=table_name, column_type="string")
                return func(func_name, col)
            elif func_name == "CONCAT":
                col = self._random_column(table_name=table_name, column_type="string")
                return func(func_name, col, self._random_literal(target_type="string"))
            elif func_name == "SUBSTR":
                col = self._random_column(table_name=table_name, column_type="string")
                return func(func_name, col, exp.Literal.number(1), exp.Literal.number(10))

        elif func_category == "numeric":
            func_name = random.choice(["ABS", "ROUND"])
            col = self._random_column(table_name=table_name, column_type="integer")
            return func(func_name, col)

        elif func_category == "timestamp":
            # Simplify timestamp functions to avoid syntax issues
            func_name = "CURRENT_TIMESTAMP"
            return func(func_name)

        else:  # any
            func_name = random.choice(["CURRENT_TIMESTAMP", "CURRENT_DATE", "RANDOM", "COALESCE"])
            if func_name in ["CURRENT_TIMESTAMP", "CURRENT_DATE", "RANDOM"]:
                return func(func_name)
            else:  # COALESCE
                col = self._random_column(table_name=table_name)
                return func(func_name, col, self._random_literal())

    def _fuzzy_random_function(self, table_name=None):
        """Generate a potentially type-incompatible function call for fuzzing."""
        func_name = random.choice(["UPPER", "LOWER", "LENGTH", "SUBSTR", "CONCAT", "ABS", "ROUND",
                                 "GREATEST", "LEAST", "COALESCE", "DATEADD", "DATEDIFF"])

        if func_name in ["UPPER", "LOWER", "LENGTH"]:
            return func(func_name, self._random_column(table_name=table_name))
        elif func_name in ["ROUND", "ABS"]:
            return func(func_name, self._random_column(table_name=table_name))
        elif func_name == "CONCAT":
            return func(func_name, self._random_column(table_name=table_name), self._random_literal())
        elif func_name == "SUBSTR":
            return func(func_name, self._random_column(table_name=table_name), exp.Literal.number(1), exp.Literal.number(10))
        elif func_name in ["GREATEST", "LEAST"]:
            return func(func_name, self._random_column(table_name=table_name), self._random_literal())
        elif func_name == "COALESCE":
            return func(func_name, self._random_column(table_name=table_name), self._random_literal())
        elif func_name in ["DATEADD", "DATEDIFF"]:
            return func(func_name, exp.Literal.string("day"), self._random_literal(), self._random_column(table_name=table_name))
        else:
            return func(func_name, self._random_column(table_name=table_name))

    def generate_complex_query(self):
        """Generate a complex query with CTEs, subqueries, and advanced features."""
        # Choose complexity level based on safe_query_probability
        if random.random() < self.safe_query_probability:
            # Safer complex query - just multiple JOINs with GROUP BY
            return self._generate_safer_complex_query()
        else:
            # More aggressive complex query with CTEs and subqueries
            return self._generate_aggressive_complex_query()

    def _generate_safer_complex_query(self):
        """Generate a complex but safer query with JOINs and aggregations."""
        # Start with a base table
        table1_name = random.choice(self.table_names)
        base_table = table(table1_name)
        table1_alias = "t1"
        query = select().from_(base_table.as_(table1_alias))

        # Track table schemas for aliases
        self._current_table_schemas = {table1_alias: list(self.table_schemas[table1_name].keys())}

        # Always use GROUP BY for complexity but keep it safe
        use_group_by = True
        columns = []
        group_by_columns = []
        used_expressions = set()

        # Add safe columns first
        safe_columns = ['id', 'name', 'email', 'status', 'type']
        available_cols = [col for col in self.table_schemas[table1_name].keys() if col in safe_columns]
        if not available_cols:
            available_cols = list(self.table_schemas[table1_name].keys())[:2]

        # Add a safe grouping column
        group_col = column(available_cols[0], table=table1_alias)
        columns.append(group_col)
        group_by_columns.append(group_col)
        used_expressions.add(group_col.sql(dialect="snowflake"))

        # Add aggregate functions
        agg_functions = ["COUNT", "MAX", "MIN"]
        for _ in range(random.randint(1, 2)):
            agg_func = random.choice(agg_functions)
            if agg_func == "COUNT":
                col = func("COUNT", exp.Star())
            else:
                target_col = column('id', table=table1_alias)  # Use safe numeric column
                col = func(agg_func, target_col)

            col_str = col.sql(dialect="snowflake")
            if col_str not in used_expressions:
                columns.append(col)
                used_expressions.add(col_str)

        # Add one JOIN for complexity
        table2_name = random.choice([t for t in self.table_names if t != table1_name])
        join_table = table(table2_name)
        table2_alias = "t2"

        # Use safe join condition (always on id)
        join_condition = column('id', table=table1_alias).eq(column('id', table=table2_alias))
        query = query.join(join_table.as_(table2_alias), on=join_condition, join_type="LEFT")

        query = query.select(*columns)

        # Add simple WHERE condition
        where_col = column('id', table=table1_alias)
        where_condition = exp.GT(this=where_col, expression=exp.Literal.number(0))
        query = query.where(where_condition)

        # Add GROUP BY
        query = query.group_by(*group_by_columns)

        # Add simple HAVING
        count_func = func("COUNT", exp.Star())
        having_condition = exp.GT(this=count_func, expression=exp.Literal.number(0))
        query = query.having(having_condition)

        # Add ORDER BY
        query = query.order_by(group_by_columns[0])

        # Add LIMIT for safety
        query = query.limit(100)

        # Clean up tracking
        if hasattr(self, '_current_table_schemas'):
            delattr(self, '_current_table_schemas')

        return query

    def _generate_aggressive_complex_query(self):
        """Generate a more aggressive complex query with CTEs and subqueries."""
        # Generate a CTE with a complex base query
        cte_query = self._generate_complex_base_query()
        cte_name = "cte_data"

        # Main query that uses the CTE
        main_query = select(exp.Star()).from_(table(cte_name))

        # Add complex conditions (but fewer than before)
        if random.choice([True, False]):
            # Only add conditions 50% of the time
            conditions = []
            for _ in range(random.randint(1, 1)):  # Reduced from 1-2 to just 1
                if random.choice([True, False]):
                    # Subquery condition - wrap in parentheses using Subquery expression
                    subquery = select(func("MAX", column('id'))).from_(self._random_table())
                    subquery_expr = exp.Subquery(this=subquery)
                    conditions.append(exp.GT(this=column('id'), expression=subquery_expr))
                else:
                    # Simple condition instead of CASE
                    conditions.append(exp.GT(this=column('id'), expression=exp.Literal.number(0)))

            if conditions:
                combined = conditions[0]
                for cond in conditions[1:]:
                    combined = and_(combined, cond)
                main_query = main_query.where(combined)

        # Add LIMIT for safety
        main_query = main_query.limit(50)

        # Combine with CTE
        final_query = main_query.with_(cte_name, as_=cte_query)

        return final_query

    def _generate_complex_base_query(self):
        """Generate a complex base query for use in CTEs."""
        # Start with a base table
        table1_name = random.choice(self.table_names)
        base_table = table(table1_name)
        table1_alias = "t1"
        query = select().from_(base_table.as_(table1_alias))

        # Track table schemas for aliases
        self._current_table_schemas = {table1_alias: list(self.table_schemas[table1_name].keys())}

        # Decide if this will be a GROUP BY query
        use_group_by = random.choice([True, False])

        # Add columns (ensure uniqueness by tracking used column expressions)
        columns = []
        group_by_columns = []
        used_expressions = set()

        for _ in range(random.randint(2, 4)):
            attempts = 0
            while attempts < 10:  # Prevent infinite loop
                if use_group_by and random.choice([True, False, False]):
                    # For GROUP BY queries, prefer aggregate functions
                    if random.choice([True, False]):
                        col = func("COUNT", exp.Star())
                    else:
                        agg_func = random.choice(["COUNT", "SUM", "AVG", "MAX", "MIN"])
                        base_col = self._random_column(table1_alias, table1_name)
                        col = func(agg_func, base_col)
                elif random.choice([True, False]):
                    col = self._random_column(table1_alias, table1_name)
                    # If using GROUP BY, this column must be in GROUP BY
                    if use_group_by:
                        group_by_columns.append(col)
                else:
                    col = self._random_function(table1_name)

                # Convert to string to check for duplicates
                col_str = col.sql(dialect="snowflake")
                if col_str not in used_expressions:
                    columns.append(col)
                    used_expressions.add(col_str)
                    break
                attempts += 1

        # Add one JOIN for complexity (simplified)
        table2_name = None

        # Add JOIN only 50% of the time
        if random.choice([True, False]):
            table2_name = random.choice([t for t in self.table_names if t != table1_name])
            join_table = table(table2_name)
            table2_alias = "t2"

            # Track the second table schema
            self._current_table_schemas[table2_alias] = list(self.table_schemas[table2_name].keys())

            # Use safe join condition (always on id)
            join_condition = column('id', table=table1_alias).eq(column('id', table=table2_alias))
            query = query.join(join_table.as_(table2_alias), on=join_condition, join_type="LEFT")

            # Add one safe column from the joined table
            if 'id' in self.table_schemas[table2_name]:
                col = column('id', table=table2_alias)
                col_str = col.sql(dialect="snowflake")
                if col_str not in used_expressions:
                    columns.append(col)
                    used_expressions.add(col_str)

        query = query.select(*columns)

        # Add simpler WHERE clause
        if random.choice([True, False]):
            # Only add WHERE 50% of the time
            where_col = column('id', table=table1_alias)
            where_condition = exp.GT(this=where_col, expression=exp.Literal.number(0))
            query = query.where(where_condition)

        # Add GROUP BY if planned
        if use_group_by and group_by_columns:
            query = query.group_by(*group_by_columns)

            # Add simple HAVING clause
            if random.choice([True, False]):
                count_func = func("COUNT", exp.Star())
                having_condition = exp.GT(this=count_func, expression=exp.Literal.number(0))
                query = query.having(having_condition)

        # Add simple ORDER BY
        if random.choice([True, False]) and columns:
            order_col = columns[0]  # Use first column
            query = query.order_by(order_col)

        # Add LIMIT for safety
        query = query.limit(100)


        # Add window functions for complexity
        if random.choice([True, False]):
            window_func = random.choice(['ROW_NUMBER', 'RANK', 'DENSE_RANK', 'LAG', 'LEAD'])
            if window_func in ['LAG', 'LEAD']:
                col = func(window_func, self._random_column(table1_alias, table1_name))
            else:
                col = func(window_func)
            # Add OVER clause
            over_clause = f'OVER (ORDER BY {self._random_column(table1_alias, table1_name).sql(dialect='snowflake')})'
            columns.append(exp.Anonymous(this=f'{window_func}() {over_clause}'))
        


        # Add window functions for complexity
        if random.choice([True, False]):
            window_func = random.choice(["ROW_NUMBER", "RANK", "DENSE_RANK", "LAG", "LEAD"])
            if window_func in ["LAG", "LEAD"]:
                col = func(window_func, self._random_column(table1_alias, table1_name))
            else:
                col = func(window_func)
            # Add OVER clause
            over_clause = f"OVER (ORDER BY {self._random_column(table1_alias, table1_name).sql(dialect='snowflake')})"
            columns.append(exp.Anonymous(this=f"{window_func}() {over_clause}"))
            


        # Add window functions for complexity
        if random.choice([True, False]):
            window_func = random.choice(["ROW_NUMBER", "RANK", "DENSE_RANK", "LAG", "LEAD"]);
            if window_func in ["LAG", "LEAD"]:
                col = func(window_func, self._random_column(table1_alias, table1_name))
            else:
                col = func(window_func)
            # Add OVER clause
            over_clause = f"OVER (ORDER BY {self._random_column(table1_alias, table1_name).sql(dialect='snowflake')})"
            columns.append(exp.Anonymous(this=f"{window_func}() {over_clause}"))
            

        # Clean up tracking
        if hasattr(self, '_current_table_schemas'):
            delattr(self, '_current_table_schemas')

        return query


def random_query_generator(database: str = "embucket",
                          db_schema: str = "public",
                          safe_query_probability: float = 0.4) -> str:
    """
    Generate a random complex SQL query using SQLGlot with Snowflake syntax support.

    Args:
        database: Database name to use for fully qualified table names
        db_schema: Schema name to use for fully qualified table names
        safe_query_probability: Probability (0.0-1.0) of generating safer, more likely to succeed queries

    Returns:
        Generated complex SQL query string in Snowflake dialect
    """
    generator = RandomQueryGenerator(database=database, schema=db_schema)
    generator.safe_query_probability = safe_query_probability

    try:
        # Always generate complex queries
        query = generator.generate_complex_query()

        # Generate SQL in Snowflake dialect
        sql_query = query.sql(dialect="snowflake", pretty=True)

        # Validate the generated SQL by trying to parse it
        try:
            sqlglot.parse_one(sql_query, dialect="snowflake")
            return sql_query
        except Exception as parse_error:
            # Fall back to a simple valid query
            fallback_query = select(exp.Star()).from_(table("users")).limit(10)
            sql_query = fallback_query.sql(dialect="snowflake")
            return sql_query

    except Exception as e:
        # Fallback to a simple query with simple table name
        fallback_query = select(exp.Star()).from_(table("users")).limit(10)
        sql_query = fallback_query.sql(dialect="snowflake")
        return sql_query
