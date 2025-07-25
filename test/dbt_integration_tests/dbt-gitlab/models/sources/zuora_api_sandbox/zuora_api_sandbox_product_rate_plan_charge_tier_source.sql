WITH source AS (

    SELECT *
    FROM {{ source('zuora_api_sandbox', 'dup_zuora_api_sandbox_stitch_productrateplanchargetier') }}

), renamed AS (

    SELECT 
      productrateplanchargeid AS product_rate_plan_charge_id,
      currency                AS currency,
      price                   AS price
    FROM source
    
)

SELECT *
FROM renamed
