WITH source AS (

    SELECT *
    FROM {{ source('zuora_api_sandbox', 'dup_zuora_api_sandbox_stitch_rateplanchargetier') }}

), renamed AS (

    SELECT 
      rateplanchargeid        AS rate_plan_charge_id,
      productrateplanchargeid AS product_rate_plan_charge_id,
      price,
      currency
    FROM source
    
)

SELECT *
FROM renamed
