type == "object"
and (.expected_owner | type == "string" and length > 0)
and (.bootstrap_unregistered | type == "array")
and (.bootstrap_unregistered | length) == (.bootstrap_unregistered | unique | length)
and all(.bootstrap_unregistered[];
  . as $crate | type == "string" and ($order[0] | index($crate) != null))
